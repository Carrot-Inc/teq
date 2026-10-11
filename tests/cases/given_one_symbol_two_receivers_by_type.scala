// One given read on two objects is ranked by its types seen from each object before the objects
// themselves (dotty's `Applications.compare`: `compareWithTypes`, `comparePrefixes` on a tie):
// `A.result: Box[A]` is as good as `B.result: Box[B]` and not the reverse, so `A`'s is taken.
package boxes:
  class Box[+T](val tag: String)
  trait G[T]:
    def name: String
    given result: Box[T] = new Box[T](name)
  class A
  object A extends G[A]:
    def name = "A"
  class B extends A
  object B extends G[B]:
    def name = "B"
  def show(): Unit = println(summon[Box[A | B]].tag)

package ties:
  class I[X](val tag: String)
  trait A:
    def name: String = "A"
    given I[B] = I[B](name)
  object A extends A
  trait B extends A:
    override def name: String = "B"
  object B extends B
  def show(): Unit = println(summon[I[B]].tag)

@main def run(): Unit =
  boxes.show()
  ties.show()
