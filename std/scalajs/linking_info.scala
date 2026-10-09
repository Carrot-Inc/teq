// Scala.js's `LinkingInfo`: the link-time properties are answered by the compiler from the build,
// `productionMode` by `--release`, `moduleKind` by the output (ES modules for `--split` and for a
// file with JavaScript imports or exports).
package scala.scalajs

object LinkingInfo:
  inline def productionMode: Boolean = linkTimePropertyBoolean("core/productionMode")
  inline def developmentMode: Boolean = !productionMode
  inline def esVersion: Int = linkTimePropertyInt("core/esVersion")
  @deprecated("use esVersion >= ESVersion.ES2015 instead", "1.6.0")
  inline def assumingES6: Boolean = esVersion >= ESVersion.ES2015
  inline def useECMAScript2015Semantics: Boolean = linkTimePropertyBoolean("core/useECMAScript2015Semantics")
  inline def moduleKind: Int = linkTimePropertyInt("core/moduleKind")
  inline def isWebAssembly: Boolean = linkTimePropertyBoolean("core/isWebAssembly")
  inline def linkerVersion: String = linkTimePropertyString("core/linkerVersion")
  inline def linkTimeIf[T](cond: Boolean)(inline thenp: T)(inline elsep: T): T = if cond then thenp else elsep

  object ESVersion:
    final val ES5_1 = 5
    final val ES2015 = 6
    final val ES2016 = 7
    final val ES2017 = 8
    final val ES2018 = 9
    final val ES2019 = 10
    final val ES2020 = 11
    final val ES2021 = 12

  object ModuleKind:
    final val NoModule = 1
    final val ESModule = 2
    final val CommonJSModule = 3

  private[scalajs] inline def linkTimePropertyBoolean(inline name: String): Boolean = compiletime.error("a link-time property")
  private[scalajs] inline def linkTimePropertyInt(inline name: String): Int = compiletime.error("a link-time property")
  private[scalajs] inline def linkTimePropertyString(inline name: String): String = compiletime.error("a link-time property")
