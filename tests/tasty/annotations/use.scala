// The downstream of the annotations' check (tests/tasty.sh, "The annotations"): what scalac reads
// of lib.scala's annotations, and the deprecation warnings its uses get under -deprecation.
package use

@main def run(): Unit =
  inspect.Inspect.annotations[annot.Lib]
  inspect.Inspect.annotations[annot.TestBase]
  inspect.Inspect.annotations[annot.Box[Int]]
  inspect.Inspect.annotations[annot.Old]
  inspect.Inspect.annotations[annot.Show[Int]]
  inspect.Inspect.annotations[annot.FieldSetter]
  inspect.Inspect.moduleAnnotations("annot.Obj")
  inspect.Inspect.moduleAnnotations("annot.Givens")
  inspect.Inspect.moduleAnnotations("annot.Level")
  inspect.Inspect.moduleAnnotations("annot.Implicits")
  inspect.Inspect.moduleAnnotations("annot.Reordered")
  inspect.Inspect.annotations[annot.Secondary]
  inspect.Inspect.annotations[annot.Stable]
  inspect.Inspect.moduleAnnotations("annot.lib$package")
  val l = new annot.Lib(1, 2, 3, 4)
  println(l.old + l.bare + l.since + l.neu)
  println(new annot.Old)
