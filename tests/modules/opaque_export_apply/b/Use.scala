package opaqueclient
import opaqueexports.*

object Aliases:
  val names: Labels.type = Labels
object Client:
  import Access.entry
  def result: Int = entry(2, 3)

@main def run(): Unit =
  println(Client.result)
  println(Library.entry(1))
  val entry: Label[Int] = Library.entry
  println(entry(4))
  val throughAlias: Aliases.names.Name[Int] = Library.entry
  println(throughAlias(6))
