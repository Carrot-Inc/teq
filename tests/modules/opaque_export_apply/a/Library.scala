package opaqueexports

trait Names[Top]:
  opaque type Name[A <: Top] = String
  def named[A <: Top](text: String): Name[A] = text
  extension [A <: Top](name: Name[A])
    def apply(values: Int*): Int = name.length + values.size

object Labels extends Names[Any]
export Labels.{Name as Label}

object Library:
  def entry: Label[Int] = Labels.named[Int]("node")

trait Facade:
  export Library.entry

object Access extends Facade
