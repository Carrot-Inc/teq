/*
rules = ["file:../../support/NoDummy.scala"]
*/
// An ok anchor trailing the last statement of an object's colon body is that statement's `endComment` alone, though
// the body, the template and the object end where it does: scalameta's parser shares a trailing comment from the tree
// that owns it up to a block (`Template.Body`), which shares it with none of the trees around it, so the object's
// other statements are linted.
object OkLastStatement:
  val aDummy = 1 // assert: NoDummy
  val bDummy = 2 // scalafix:ok NoDummy
