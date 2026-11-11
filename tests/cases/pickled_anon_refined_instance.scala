trait Cell:
  type Elem
  def value: Elem

abstract class Labelled(val label: String) extends Cell

object Main:
  def cell[A](a: A): Cell { type Elem = A } =
    new Cell:
      type Elem = A
      def value: A = a

  def main(args: Array[String]): Unit =
    val inferred = new Cell:
      type Elem = Int
      def value = 7
    val erased: Cell = new Cell:
      type Elem = String
      def value = "erased"
    val explicit = (new Cell { type Elem = String; def value = "explicit" }: Cell)
    val labelled = new Labelled("value"):
      type Elem = String
      def value = "labelled"
    println(inferred.value + 1)
    println(cell("generic").value)
    println(erased.value)
    println(explicit.value)
    println(labelled.label + ":" + labelled.value)
