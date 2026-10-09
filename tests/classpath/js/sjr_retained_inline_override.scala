// jars: scala-library scalajs-react
// An inline override of a jar runs, where it is dispatched, the body scalac retained for it
// (`modifyComponentName$retainedBody`), already expanded; a call on the object itself expands the
// inline body. scalajs-react 4.0.0's `ScalaJsReactConfig.Instance`, whose inline body reaches a
// macro warning with a message that is no constant in a case scalac never keeps. The expectation
// is scalac 3.8.4's on Scala.js.
package retained

import japgolly.scalajs.react.ScalaJsReactConfig

@main def main(): Unit =
  val c: ScalaJsReactConfig = ScalaJsReactConfig.Instance
  println(c.modifyComponentName("Named"))
  println(c.automaticComponentName("a.b.Auto"))
  println(c.reusabilityOverride == null)
  println(ScalaJsReactConfig.Instance.modifyComponentName("Direct"))
