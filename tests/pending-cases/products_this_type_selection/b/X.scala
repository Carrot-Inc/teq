// A downstream of a/ as tests/modules builds it. `Outer.f` is an inline method whose body ascribes to `this.T`. scalac
// 3.8.4 prints the .expected, and so does each of teq's runs. scalac's class binds the receiver for the expansion of
// `o.f(4)` (javap: `aload_1; astore_2`), its `this` proxy, which the type tree names (`Inliner`'s `dropUnusedDefs`
// counts it), and so does teq's whole build; teq's build over a/'s products drops the binding, so b/'s classes and
// bundles differ between the builds (tests/modules' classes, subset, bundle). Not the qualifier: teq pickled it as
// QUALTHIS(IDENTtpt Outer) where scalac writes THIS (reviewer #14's O5), and since the kept type tree's `this` keeps
// its qualifier as written (push 11) it writes THIS, and the binding is dropped all the same.
// Before the SELECTtpt port (52720e53bf) the products build panicked on the type selection. Found by reviewer #14 of
// the workers exports helper (push 9).
// The mechanism: the converter reads the ascription's type `TypeRef(This(Outer), T)` through the loader's map, which
// gives the alias's `Int` (`conv_type_inner`'s `map_type`), so the expansion's type tree names no `this`, and the
// count of `dropUnusedDefs` (`updateTermRefCounts` over a TypTree's type) finds no use of the receiver's proxy; the
// whole build's source typer keeps `this.T`, mapped to the proxy's `T`. dotty's unpickler keeps the TypeRef. Master
// efddb38de3 drops the binding in both builds, which agree: a regression of the builds' agreement, the loader's
// dealiasing of a member alias a representation seam, queued to the representation part.
package ttb

@main def run(): Unit =
  val o = tta.Outer()
  println(o.f(4))
  println(o.g(5))
