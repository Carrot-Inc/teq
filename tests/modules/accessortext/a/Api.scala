package textprobe

// A string literal spelled like an inline accessor's name: the pickle writes its text as a
// simple name (dotty's `TreePickler.pickleConstant`, `stringValue.toTermName`), never the
// derived `INLINEACCESSOR` a generated accessor's symbol has, so it reads back as written.
object Api:
  inline def label: String = "inline$owner$$member"
  inline def plain: String = "inline$plain"
