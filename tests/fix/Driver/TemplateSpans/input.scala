/*
rules = ["file:../../support/Templates.scala"]
*/
// A template spans its braces and an empty body (scalameta's, the review of `a24540c28`): the empty one is replaced
// through `replaceTree`, and each is reported with its text.
object EmptyBody {} // assert: Templates.span
object Full { val x = 1 } /* assert: Templates.span
            ^^^^^^^^^^^^^
{ val x = 1 }
*/
class Base // assert: Templates.span
class Child extends Base { def y = 2 } /* assert: Templates.span
                    ^^^^^^^^^^^^^^^^^^
Base { def y = 2 }
*/
