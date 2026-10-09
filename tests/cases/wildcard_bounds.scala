// A bounded wildcard in an invariant position keeps its bounds: its capture reads as the upper
// bound, takes the lower one, and a value conforms when its argument lies between them.
trait Animal:
  def name: String

final case class Dog(name: String) extends Animal
final case class Cat(name: String) extends Animal

final class Box[A](var value: A)

object Main:
  def names(s: Set[? <: Animal]): List[String] = s.toList.map(_.name).sorted
  def first(b: Box[? <: Animal]): String = b.value.name
  def pick(it: Iterator[Animal]): String = it.map(_.name).toList.sorted.mkString(",")
  def pick(s: Set[? <: Animal]): String = pick(s.iterator)
  def render(db: Map[? <: Animal, Set[? <: Animal]]): String =
    db.toList.sortBy(_._1.name).map { case (k, v) => s"${k.name} -> ${v.toList.map(_.name).sorted.mkString(",")}" }.mkString("; ")
  def reset(b: Box[? >: Dog]): Unit = b.value = Dog("rex")

  def main(args: Array[String]): Unit =
    val dogs: Set[Dog] = Set(Dog("b"), Dog("a"))
    println(names(dogs))
    println(first(Box(Cat("tom"))))
    println(pick(dogs))
    println(pick(Iterator(Cat("z"), Dog("y"))))
    println(render(Map(Dog("d") -> Set(Cat("c"), Cat("a")), Cat("b") -> Set.empty[Dog])))
    val any = Box[Animal](Cat("x"))
    reset(any)
    println(any.value)
    val bs: Box[? <: Animal] = Box(Dog("w"))
    println(bs.value.name)
