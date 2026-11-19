// teq: --std=scala-library
object Main:
  val x = summon[Int]
// Under scala-library's std, `summon` is Predef's, and a missing given's owner phrase is scalac 3.8.4's line whole
// (`… of method summon in object Predef`, at one column less). The lean std declares `summon` at the top level of
// package scala, and says so (`in package scala`): the row "a lean std definition is printed with scala-library's
// owner".
// expect: 3:22: error: No given instance of type Int was found for parameter x of method summon in object Predef
// expect: 1 error found
