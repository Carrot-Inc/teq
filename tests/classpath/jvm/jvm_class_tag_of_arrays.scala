// jars: scala-library
// std: lean scala-library
// The `ClassTag` of a union or an intersection with arrays in it, by scalac's erasure: two
// arrays of references are an array of what their elements are together, two of one primitive
// are that array, any other union with an array is `Object`, and in an intersection an array
// comes before what is none. The arrays made by the tags are of those kinds.
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
  println("Array[Int] | Array[String]: " + tag[Array[Int] | Array[String]])
  println("Array[Int] | Array[Int]: " + tag[Array[Int] | Array[Int]])
  println("Array[String] | Array[A]: " + tag[Array[String] | Array[A]])
  println("Array[B] | Array[C]: " + tag[Array[B] | Array[C]])
  println("Array[Int] | Array[Long]: " + tag[Array[Int] | Array[Long]])
  println("Array[Int] | String: " + tag[Array[Int] | String])
  println("Array[Int] & java.io.Serializable: " + tag[Array[Int] & java.io.Serializable])
  println("java.io.Serializable & Array[Int]: " + tag[java.io.Serializable & Array[Int]])
  println("Array[A] & Array[B]: " + tag[Array[A] & Array[B]])
  println("Array[Array[Int]] | Array[Array[String]]: " + tag[Array[Array[Int]] | Array[Array[String]]])
  println("Array[B | C]: " + tag[Array[B | C]])
  println("Array[A & T1]: " + tag[Array[A & T1]])
  println(Array.empty[Array[Int] | Array[String]].getClass.getSimpleName + " " + Array.empty[Array[Int] | Array[String]].length)
  println(new Array[Array[B] | Array[C]](2).getClass.getSimpleName + " " + new Array[Array[Int] & java.io.Serializable](1).getClass.getSimpleName)
  println(List[B | C](new B, new C).toArray.getClass.getSimpleName + " " + Array[A & T1](new D, new E).getClass.getSimpleName)
