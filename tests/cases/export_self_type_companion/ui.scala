package xui
case class Esm(key: String)
trait AssetHelper:
  export xui.Esm
trait I18nHelper:
  def lang: String = "en"
