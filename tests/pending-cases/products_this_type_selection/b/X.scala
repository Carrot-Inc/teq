// A downstream of a/ as tests/modules builds
// it. `Outer.f` is an inline method whose
// body ascribes to `this.T`. scalac 3.8.4
// prints the .expected. scalac's class
// binds the receiver for the expansion of
// `o.f(4)` (javap: `aload_1; astore_2`),
// its `this` proxy, which the type tree
// names (`Inliner`'s `dropUnusedDefs`
// counts it); teq's builds, whole and over
// a/'s products, are to bind it too, b/'s
// classes and bundles the same between the
// builds (tests/modules' classes, subset,
// bundle). dotty's unpickler keeps the
// ascription's type, `TypeRef(This(Outer), T)`.
// Queued to the representation part.
package ttb

@main def run(): Unit =
  val o = tta.Outer()
  println(o.f(4))
  println(o.g(5))
