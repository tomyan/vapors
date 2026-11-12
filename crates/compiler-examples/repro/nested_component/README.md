# A component tag inside another component's children does not compile

Not built: these files sit outside `src`, so the crate builds without
them. To reproduce (blocks one-scene R3a1, `docs/plans/3d-2.md` row 4):

```
cp crates/compiler-examples/repro/nested_component/*.wisp crates/compiler-examples/src/
cargo build -p vapors-compiler-examples
# error[E0308]: mismatched types ... expected `&Rc<RefCell<D>>`, found `&Rc<RefCell<dyn Dom>>`
rm crates/compiler-examples/src/repro_*.wisp
```

`<ReproInner />` is built inside `ReproHolder`'s children snippet, where
`dom` is `&Rc<RefCell<dyn Dom>>`. The generated
`ReproInner::mount_before<D: Dom + ?Sized>(dom: &Rc<RefCell<D>>, ..)
where Rc<RefCell<D>>: DynDom` is called there, and rustc takes the
enclosing `ReproOuter::mount_before`'s where-clause
(`Rc<RefCell<D>>: DynDom` for its own `D`) as the candidate, binding the
inner `D` to the outer one. The scene components take `dom: &impl
DynDom` instead (`crates/scene3d-components`). When the compiler is
fixed, move these files into `src` with a test, and delete this
directory.
