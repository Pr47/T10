use std::any::{TypeId, Any, type_name};
use std::marker::PhantomData;
use std::mem::{MaybeUninit, ManuallyDrop, transmute};
use std::ptr::{NonNull, null_mut};
use std::sync::atomic::AtomicU8;
use std::sync::atomic::Ordering::SeqCst;

use crate::tyck::base::StaticBase;
use crate::tyck::TypeCheckInfo;
use crate::void::Void;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum GcInfo {
    Owned = 0,
    SharedWithHost = 1,
    MutSharedWithHost = 2,
    MovedToHost = 3,
    Dropped = 4,
    Null = 5,
    OnStack = 6,
}

impl GcInfo {
    pub fn from_u8(src: u8) -> GcInfo {
        match src {
            0 => GcInfo::Owned,
            1 => GcInfo::SharedWithHost,
            2 => GcInfo::MutSharedWithHost,
            3 => GcInfo::MovedToHost,
            4 => GcInfo::Dropped,
            5 => GcInfo::Null,
            6 => GcInfo::OnStack,
            _ => unreachable!()
        }
    }
}

union WrapperData<T> {
    value: ManuallyDrop<MaybeUninit<T>>,
    ptr: NonNull<T>
}

pub struct Wrapper<'a, Ta: 'a, Ts: 'static> {
    data: WrapperData<Ta>,
    gc_info: AtomicU8,
    _phantom: PhantomData<&'a Ts>
}

impl<'a, Ta: 'a, Ts: 'static> Wrapper<'a, Ta, Ts> {
    pub fn owned(data: Ta) -> Self {
        Self {
            data: WrapperData {
                value: ManuallyDrop::new(MaybeUninit::new(data))
            },
            gc_info: AtomicU8::new(GcInfo::Owned as u8),
            _phantom: PhantomData::default()
        }
    }

    pub fn shared(data: &Ta) -> Self {
        Self {
            data: WrapperData {
                ptr: unsafe { NonNull::new_unchecked(data as *const Ta as *mut Ta) }
            },
            gc_info: AtomicU8::new(GcInfo::SharedWithHost as u8),
            _phantom: PhantomData::default()
        }
    }

    pub fn mut_shared(data: &mut Ta) -> Self {
        Self {
            data: WrapperData {
                ptr: unsafe { NonNull::new_unchecked(data as *mut Ta) }
            },
            gc_info: AtomicU8::new(GcInfo::MutSharedWithHost as u8),
            _phantom: PhantomData::default()
        }
    }

    #[inline] pub unsafe fn borrow_value(&self) -> NonNull<()> {
        NonNull::new_unchecked(self.data.value.as_ptr() as *const () as *mut ())
    }

    #[inline] pub unsafe fn borrow_ptr(&self) -> NonNull<()> {
        self.data.ptr.cast()
    }

    #[inline] pub unsafe fn take_value(&mut self) -> Ta {
        ManuallyDrop::take(&mut self.data.value).assume_init()
    }

    #[inline] fn gc_info_impl(&self) -> GcInfo {
        GcInfo::from_u8(self.gc_info.load(SeqCst))
    }

    #[inline] fn set_gc_info_impl(&mut self, gc_info: GcInfo) {
        self.gc_info.store(gc_info as u8, SeqCst)
    }
}

impl<'a, Ta: 'a, Ts: 'static> Drop for Wrapper<'a, Ta, Ts> {
    fn drop(&mut self) {
        if false /* TODO use the real condition here */ {
            unsafe {
                ManuallyDrop::take(&mut self.data.value).assume_init_drop();
            }
        }
    }
}

pub type StaticWrapper<T> = Wrapper<'static, T, T>;

pub trait DynBase {
    fn dyn_type_id(&self) -> std::any::TypeId;
    fn dyn_type_name(&self) -> &'static str;
    fn dyn_tyck(&self, tyck_info: &TypeCheckInfo) -> bool;
    fn dyn_tyck_info(&self) -> TypeCheckInfo;

    fn gc_info(&self) -> GcInfo;
    fn set_gc_info(&mut self, gc_info: GcInfo);

    unsafe fn get_ptr(&self) -> NonNull<()>;

    #[cfg(not(debug_assertions))]
    unsafe fn move_out(&mut self, dest: *mut ());

    #[cfg(debug_assertions)]
    unsafe fn move_out_ck(&mut self, dest: *mut (), dest_ty: TypeId);
}

impl<'a, Ta: 'a, Ts: 'static> DynBase for Wrapper<'a, Ta, Ts> {
    fn dyn_type_id(&self) -> TypeId {
        TypeId::of::<Ts>()
    }

    fn dyn_type_name(&self) -> &'static str {
        type_name::<Ta>()
    }

    fn dyn_tyck(&self, tyck_info: &TypeCheckInfo) -> bool {
        <Void as StaticBase<Ts>>::tyck(tyck_info)
    }

    fn dyn_tyck_info(&self) -> TypeCheckInfo {
        <Void as StaticBase<Ts>>::tyck_info()
    }

    fn gc_info(&self) -> GcInfo {
         self.gc_info_impl()
    }

    fn set_gc_info(&mut self, gc_info: GcInfo) {
        self.set_gc_info_impl(gc_info)
    }

    unsafe fn get_ptr(&self) -> NonNull<()> {
        match GcInfo::from_u8(self.gc_info.load(SeqCst)) {
            GcInfo::Owned => self.borrow_value(),
            GcInfo::SharedWithHost | GcInfo::MutSharedWithHost => self.borrow_ptr(),
            GcInfo::MovedToHost => unreachable!("cannot use moved value"),
            GcInfo::Dropped => unreachable!("cannot use dropped value"),
            GcInfo::Null => unreachable!("null pointer should not occur at this layer"),
            GcInfo::OnStack => unreachable!("stack value should not occur at this layer")
        }
    }

    #[cfg(not(debug_assertions))]
    unsafe fn move_out(&mut self, dest: *mut ()) {
        let dest = (dest as *mut MaybeUninit<Ta>).as_mut().unwrap();
        dest.write(self.take_value());
    }

    #[cfg(debug_assertions)]
    unsafe fn move_out_ck(&mut self, dest: *mut (), dest_ty: TypeId) {
        debug_assert_eq!(self.gc_info(), GcInfo::Owned);
        debug_assert_eq!(dest_ty, TypeId::of::<MaybeUninit<Ts>>());
        let dest = (dest as *mut MaybeUninit<Ta>).as_mut().unwrap();
        dest.write(self.take_value());
    }
}

#[derive(Copy, Clone)]
pub enum ValueType {
    Int = 1,
    Float = 2,
    Char = 3,
    Byte = 4,
    Bool = 5,
    AnyType = 6
}

impl From<u8> for ValueType {
    fn from(src: u8) -> Self {
        match src {
            1 => ValueType::Int,
            2 => ValueType::Float,
            3 => ValueType::Char,
            4 => ValueType::Byte,
            5 => ValueType::Bool,
            6 => ValueType::AnyType,
            _ => unreachable!("invalid ValueType")
        }
    }
}

#[repr(C)]
pub union ValueData {
    pub(crate) ptr: *mut dyn DynBase,
    pub(crate) int: i64,
    pub(crate) float: f64,
    pub(crate) ch: char,
    pub(crate) byte: u8,
    pub(crate) boolean: bool
}

#[repr(C)]
pub struct Value<'a> {
    pub(crate) data: ValueData,
    tag: u8,
    _phantom: PhantomData<&'a ()>
}

const VALUE_MASK      : u8 = 0b10000000;
const NULL_MASK       : u8 = 0b01000000;
const VALUE_TYPE_MASK : u8 = 0b00000111;

impl<'a> Value<'a> {
    #[inline] pub(crate) fn new(data: ValueData, tag: u8) -> Self {
        Self {
            data,
            tag,
            _phantom: PhantomData::default()
        }
    }

    #[inline] pub fn null_ptr() -> Self {
        Self::new(ValueData { ptr: null_mut::<StaticWrapper<Void>>() as *mut dyn DynBase },
                  NULL_MASK)
    }

    #[inline] pub fn null_value(value_type: ValueType) -> Self {
        Self::new(ValueData { int: 0 }, VALUE_MASK | NULL_MASK | (value_type as u8))
    }

    #[inline] pub fn is_null(&self) -> bool {
        (self.tag & NULL_MASK) != 0
    }

    #[inline] pub fn is_value(&self) -> bool {
        (self.tag & VALUE_MASK) != 0
    }

    #[inline] pub fn is_ptr(&self) -> bool {
        (self.tag & VALUE_MASK) == 0
    }

    #[inline] pub fn type_id(&self) -> TypeId {
        if self.is_value() {
            match ValueType::from(self.tag & VALUE_TYPE_MASK) {
                ValueType::Int     => TypeId::of::<i64>(),
                ValueType::Float   => TypeId::of::<f64>(),
                ValueType::Char    => TypeId::of::<char>(),
                ValueType::Byte    => TypeId::of::<u8>(),
                ValueType::Bool    => TypeId::of::<bool>(),
                ValueType::AnyType => TypeId::of::<dyn Any>()
            }
        } else {
            if self.is_null() {
                unreachable!("should not use type_id on null value")
            } else {
                unsafe {
                    self.data.ptr.as_ref().unwrap().dyn_type_id()
                }
            }
        }
    }

    pub fn gc_info(&self) -> GcInfo {
        if self.is_value() {
            GcInfo::OnStack
        } else if self.is_null() {
            GcInfo::Null
        } else {
            unsafe {
                self.data.ptr.as_ref().unwrap().gc_info()
            }
        }
    }

    #[inline] pub unsafe fn set_gc_info(&self, gc_info: GcInfo) {
        debug_assert!(!self.is_value());
        debug_assert!(!self.is_null());
        self.data.ptr.as_mut().unwrap().set_gc_info(gc_info);
    }

    #[inline] pub unsafe fn as_ref<T>(&self) -> &'a T {
        debug_assert!(!self.is_value());
        debug_assert!(!self.is_null());
        let non_null = self.data.ptr.as_ref().unwrap().get_ptr().cast::<T>();
        transmute::<&T, &'a T>(non_null.as_ref())
    }

    #[inline] pub unsafe fn as_mut<T>(&self) -> &'a mut T {
        debug_assert!(!self.is_null());
        debug_assert!(!self.is_value());
        let mut non_null = self.data.ptr.as_ref().unwrap().get_ptr().cast::<T>();
        transmute::<&mut T, &'a mut T>(non_null.as_mut())
    }
}

impl<'a> From<*mut dyn DynBase> for Value<'a> {
    fn from(ptr: *mut dyn DynBase) -> Self {
        Self::new(ValueData { ptr }, 0)
    }
}

impl<'a> From<i64> for Value<'a> {
    fn from(int: i64) -> Self {
        Self::new(ValueData { int }, VALUE_MASK | ValueType::Int as u8)
    }
}

impl<'a> From<f64> for Value<'a> {
    fn from(float: f64) -> Self {
        Self::new(ValueData { float }, VALUE_MASK | ValueType::Float as u8)
    }
}

impl<'a> From<char> for Value<'a> {
    fn from(ch: char) -> Self {
        Self::new(ValueData { ch }, VALUE_MASK | ValueType::Char as u8)
    }
}

impl<'a> From<bool> for Value<'a> {
    fn from(boolean: bool) -> Self {
        Self::new(ValueData { boolean }, VALUE_MASK | ValueType::Bool as u8)
    }
}

impl<'a> From<u8> for Value<'a> {
    fn from(byte: u8) -> Self {
        Self::new(ValueData { byte }, VALUE_MASK | ValueType::Byte as u8)
    }
}
