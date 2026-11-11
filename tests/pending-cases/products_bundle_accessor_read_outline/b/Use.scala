// A downstream of a/ as tests/modules builds it. scalac 3.8.4 prints the .expected, and so does each of teq's runs.
// The two expansions of `add` read C's private `total` through the accessors `aroa$C$$inline$total` and its setter,
// one shape that teq's Scala.js bundles write once (`add$o…`, emit/outline.rs). The whole build's expansion reads the
// parameterless accessor as a selection its symbol marks (`needs_accessor`, typer::accessors' `rewire`), the
// products build's, re-typed from a/'s pickle, as the invocation teq gives every parameterless method
// (`apply_resolved_clauses`): the shapes' keys differ (`T_FIELD` against `T_CALL_METHOD`), so the function's name
// does (tests/modules' bundle: one file, --split and --release alike; accessorcompanion's, accessornames',
// accessorinherited's and accessors'), the output the same. Master efddb38de3's builds read the companion's members
// directly in both and agree: a regression since the whole build calls the accessors, a representation seam
// (teq's executable empty Apply of a parameterless method), queued to the representation part.
package arob

@main def run(): Unit =
  val c = new aroa.C()
  println(c.add(2) + c.add(3))
