/*
rules = ["file:../../support/DumpLayout.scala"]
*/
// A colon body's self alias is its `Self`, its name the written `self`, as scalameta's.
trait Subject:
  self: Subject =>
  val marker = 0 // assert: DumpLayout.layout
  def value = 1
