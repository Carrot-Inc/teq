// A top-level val of one package named as an object of another (scalajs-dom's deprecated
// `experimental.RequestRedirect` beside `dom.RequestRedirect`): both are read, each its own.
package pvo.dom:
  object Redirect:
    val follow: String = "follow"
    val manual: String = "manual"

package pvo.dom.experimental:
  val Redirect: pvo.dom.Redirect.type = pvo.dom.Redirect
  val Other: Int = 42

package pvo:
  object Main:
    def main(args: Array[String]): Unit =
      println(dom.Redirect.follow)
      println(dom.experimental.Redirect.manual)
      println(dom.experimental.Other)
