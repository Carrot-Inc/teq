//> using platform js
// Scala.js's link-time properties as a development build (fastLinkJS) answers them.
import scala.scalajs.LinkingInfo

@main def run(): Unit =
  println(LinkingInfo.developmentMode)
  println(LinkingInfo.productionMode)
  println(LinkingInfo.esVersion >= LinkingInfo.ESVersion.ES2015)
  println(LinkingInfo.useECMAScript2015Semantics)
  println(LinkingInfo.isWebAssembly)
  println(LinkingInfo.ModuleKind.NoModule + LinkingInfo.ModuleKind.ESModule + LinkingInfo.ModuleKind.CommonJSModule)
  println(LinkingInfo.linkTimeIf(LinkingInfo.productionMode)("production")("development"))
  if LinkingInfo.developmentMode then println("checks on")
