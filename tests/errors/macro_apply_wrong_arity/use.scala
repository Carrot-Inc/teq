// expect: use.scala:6:33: error: wrong number of arguments at inlining (while expanding macro) for (c: Coll)(s: String): Api: (Api#<init> : (c: Coll)(s: String): Api), expected: 1, found: 2
// expect: 1 error found
class Coll
class Api(c: Coll)(s: String):
  def show = s"api $s"
@main def wrongMain() = println(WrongMacro.make[Api].show)
