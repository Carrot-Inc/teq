/*
rules = ["file:../../support/UnwrapAnnotate.scala"]
*/
// A nested annotation ascription is two `Term.Annotate`s: the outer removed, the inner kept
// (the output scalafix-core 0.14.9's).
package driver.unwrapnestedannotate
object Subject {
  val x = ((1): @unchecked): @unchecked
}
