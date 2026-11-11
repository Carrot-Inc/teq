package model

// A top-level opaque type with a given in its companion, in a file and a package of its own.
trait Put[A]:
  def put(a: A): String

opaque type Id = Int
object Id:
  def apply(value: Int): Id = value
  extension (id: Id) def value: Int = id
  given Put[Id] = new Put[Id] { def put(a: Id): String = "id:" + a.value }
