package xweb
package ui

// A companion an exported alias of the self type's class stands for is in scope in the body, a
// member of the self type found through `this` (dotty's findRef; lila's AssetFullHelper,
// `self: AssetHelper & I18nHelper =>`, applying the `Esm` that `AssetHelper` exports).
trait AssetFullHelper:
  self: xui.AssetHelper & xui.I18nHelper =>
  def tag: Option[Esm] = Some(Esm("round." + lang))

object H extends AssetFullHelper, xui.AssetHelper, xui.I18nHelper

@main def run(): Unit = println(H.tag)
