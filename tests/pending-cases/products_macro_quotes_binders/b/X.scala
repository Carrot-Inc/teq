// A downstream of a/ as tests/modules builds it. Two macro implementations of a/ each take their own `using Quotes`
// (`x$1`) and read `TypeRepr.of[T]`. scalac 3.8.4 prints the .expected, and so does teq's interpreter run. scalac
// reading teq's pickle of a/ (tests/modules' readtasty-a, `-Ycheck`) fails: "Type Mismatch (while checking adapt):
// Found: x$1.reflect.TypeRepr, Required: x$1².reflect.TypeRepr", a tree of one method typed by the other's binder.
// With `other` removed it passes. Cause not located. The same on integration b602956824; visible in
// tests/modules/macroderive once the SELECTtpt port lets its products build. The rows macro cases share fail as theirs
// do (check, scalac-b, scalac-run, scalac-lean-b).
package qrb

@main def run(): Unit =
  println(qra.Mac.nameOf[Int])
