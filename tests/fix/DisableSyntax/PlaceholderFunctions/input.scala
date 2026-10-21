/*
rules = DisableSyntax
DisableSyntax.noUniversalEquality = true
*/
package test.disableSyntax

object PlaceholderFunctions {
  val xs = List(1, 2, 3)
  val more = xs.filter(_ > 1).map(_ + 1)
  val ones = xs.filter(_ == 1) // assert: DisableSyntax.==
  val others = xs.filter((_: Int) != 1) // assert: DisableSyntax.!=
  val pairs = xs.zip(xs).map(_ * _)
}
