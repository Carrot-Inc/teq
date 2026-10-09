// The `ClassTag` of a union or an intersection carries the class of scalac's erasure of the
// type: for a union a class or a trait that both sides derive from (of those in the first
// side's linearisation up to its first trait, the last that none of the others derives from),
// for an intersection the side that comes first in scalac's order (a primitive before a
// reference, a class before a trait, what derives before what it derives from, and else the
// name that comes first).
import scala.reflect.ClassTag

class A
class B extends A
class C extends A
trait T1
trait T2
trait T3 extends T1
class D extends A with T1
class E extends A with T1 with T2
class F extends T1 with T2
class G extends T2 with T1
final class H
trait U1 extends T1 with T2
trait U2 extends T2 with T1

def tag[X](using t: ClassTag[X]): String = t.runtimeClass.getName

@main def run(): Unit =
  println("B | C: " + tag[B | C])
  println("C | B: " + tag[C | B])
  println("A | B: " + tag[A | B])
  println("A & B: " + tag[A & B])
  println("B & A: " + tag[B & A])
  println("B & C: " + tag[B & C])
  println("C & B: " + tag[C & B])
  println("D | E: " + tag[D | E])
  println("E | D: " + tag[E | D])
  println("F | E: " + tag[F | E])
  println("E | F: " + tag[E | F])
  println("F | G: " + tag[F | G])
  println("G | F: " + tag[G | F])
  println("U1 | U2: " + tag[U1 | U2])
  println("U2 | U1: " + tag[U2 | U1])
  println("T1 | T2: " + tag[T1 | T2])
  println("T3 | T1: " + tag[T3 | T1])
  println("T3 | T2: " + tag[T3 | T2])
  println("A & T1: " + tag[A & T1])
  println("T1 & A: " + tag[T1 & A])
  println("T1 & T2: " + tag[T1 & T2])
  println("T2 & T1: " + tag[T2 & T1])
  println("T3 & T1: " + tag[T3 & T1])
  println("T1 & T3: " + tag[T1 & T3])
  println("H & T1: " + tag[H & T1])
  println("B | C | D: " + tag[B | C | D])
  println("B | D | H: " + tag[B | D | H])
  println("A & T1 & T2: " + tag[A & T1 & T2])
  println("T2 & T1 & A: " + tag[T2 & T1 & A])
  println("(B | C) & T1: " + tag[(B | C) & T1])
  println("(B & T1) | (C & T1): " + tag[(B & T1) | (C & T1)])
  println("String | Int: " + tag[String | Int])
  println("Int | Long: " + tag[Int | Long])
  println("Int | Int: " + tag[Int | Int])
  println("String | Null: " + tag[String | Null])
  println("Null | A: " + tag[Null | A])
  println("A | Nothing: " + tag[A | Nothing])
  println("1 | 2: " + tag[1 | 2])
  println("\"a\" | \"b\": " + tag["a" | "b"])
  println("1 | \"a\": " + tag[1 | "a"])
  println("Some[Int] | None.type: " + tag[Some[Int] | None.type])
  println("Int | Boolean: " + tag[Int | Boolean])
  println("Unit | Int: " + tag[Unit | Int])
  println("Any: " + tag[Any] + " AnyRef: " + tag[AnyRef] + " AnyVal: " + tag[AnyVal] + " Unit: " + tag[Unit])
