package app

trait Action:
  def run(): Int

object Actions:
  // The anonymous class an expansion makes belongs to the caller's file: under `--own` it is
  // written with the caller's class files, not left to the build of this root. Its name carries
  // the positions of the body and of the call, so it stays ahead of the lines the suite edits.
  inline def make(): Action = new Action:
    def run(): Int = 42

class Item(val name: String, val price: Int):
  def describe: String = name + " costs " + price

object Catalog:
  val items: List[Item] = List(Item("pen", 3), Item("ink", 5))
  def total: Int = items.map(_.price).sum
