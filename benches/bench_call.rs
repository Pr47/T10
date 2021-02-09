#![feature(test)]
#![feature(core_intrinsics)]

extern crate test;

use test::Bencher;

use t10::func::{RustFunction, RustCallable};
use t10::data::{StaticWrapper, DynBase, Value};
use std::marker::PhantomData;

struct S(i32);

fn bar(x: &mut S, y: &S) -> i64 {
    (x.0 + y.0) as i64
}

fn baz(x: i64, y: i64) -> i64 {
    x + y
}

#[bench]
fn bench_simple_call(b: &mut Bencher) {
    let s1 = Box::leak(Box::new(StaticWrapper::owned(S(0)))) as *mut dyn DynBase;
    let s2 = Box::leak(Box::new(StaticWrapper::owned(S(4)))) as *mut dyn DynBase;
    let v1 = Value::from(s1);
    let v2 = Value::from(s2);
    let f = RustFunction { f: bar, _phantom: PhantomData::default() };

    b.iter(|| {
        unsafe {
            for _ in 0..1000 {
                for _ in 0..1000 {
                    let _x = f.call_prechecked(&[v1, v2]).unwrap().value_typed_data.inner.int;
                }
            }
        }
    })
}

/*
#[bench]
fn bench_calc(b: &mut Bencher) {
    let mut s1 = S(13);
    let s2 = S(5);
    b.iter(|| {
        for _ in 0..1000 {
            for _ in 0..1000 {
                unsafe {
                    let _x = bar(
                        &mut volatile_load(&mut s1 as *mut S),
                        &volatile_load(&s2 as *const S as *mut S)
                    );
                }
            }
        }
    })
}
*/

#[bench]
fn bench_simple_call2(b: &mut Bencher) {
    let f = RustFunction { f: baz, _phantom: PhantomData::default() };
    b.iter(|| {
        for i in 0..1000i64 {
            for j in 0..1000i64 {
                let v1 = Value::from(i);
                let v2 = Value::from(j);
                unsafe {
                    let _x = f.call_prechecked(&[v1, v2]).unwrap().value_typed_data.inner.int;
                }
            }
        }
    })
}
