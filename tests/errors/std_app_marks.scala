// expect: 7:21: warning: trait App in package scala is deprecated since 3.8.0: Support for trait App is deprecated in Scala 3. Please refer to https://docs.scala-lang.org/scala3/book/methods-main-methods.html.
// expect: 8:3: warning: method delayedInit in trait App is deprecated since 2.11.0: the delayedInit mechanism will disappear
// expect: 9:15: warning: trait DelayedInit in package scala is deprecated since 2.11.0: DelayedInit semantics can be surprising. Support for `App` will continue. See the release notes for more details: https://github.com/scala/scala/releases/tag/v2.11.0
// expect: 3 warnings found, errors under --werror
// teq: --werror --deprecation
// The deprecation marks of `App`, of its `delayedInit` and of `DelayedInit` are scala-library's, message and version.
object Test extends App {
  delayedInit { println("block") }
  def keep(d: DelayedInit): Unit = ()
}
