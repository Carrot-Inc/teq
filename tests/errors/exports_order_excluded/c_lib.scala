package lib

trait TagsBase:
  def main: String = "main-tag"
  def cls: String = "cls"

object Tags extends TagsBase

trait OptionSyntax:
  extension [A](a: A) def some: Option[A] = Some(a)
  extension [A](a: A) def none: Option[A] = None

object Syntax extends OptionSyntax
