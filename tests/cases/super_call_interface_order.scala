//> using platform jvm
// split-watch: jvm
import java.util.Collection
import java.util.function.{IntPredicate, IntUnaryOperator, Predicate}

trait Marker

class Parent extends IntPredicate with IntUnaryOperator:
  def test(x: Int): Boolean = x > 0
  def applyAsInt(x: Int): Int = x + 1

class MethodOneLinePredicateFirst extends Parent with Marker with IntPredicate with IntUnaryOperator:
  def both(): Unit =
    super[IntPredicate].and(this); super[IntUnaryOperator].andThen(this)
    ()

class MethodOneLineOperatorFirst extends Parent with Marker with IntPredicate with IntUnaryOperator:
  def both(): Unit =
    super[IntUnaryOperator].andThen(this); super[IntPredicate].and(this)
    ()

class MethodTwoLinesPredicateFirst extends Parent with Marker with IntPredicate with IntUnaryOperator:
  def both(): Unit =
    super[IntPredicate].and(this)
    super[IntUnaryOperator].andThen(this)
    ()

class MethodTwoLinesOperatorFirst extends Parent with Marker with IntPredicate with IntUnaryOperator:
  def both(): Unit =
    super[IntUnaryOperator].andThen(this)
    super[IntPredicate].and(this)
    ()

class ConstructorOneLinePredicateFirst extends Parent with Marker with IntPredicate with IntUnaryOperator:
  super[IntPredicate].and(this); super[IntUnaryOperator].andThen(this)

class ConstructorOneLineOperatorFirst extends Parent with Marker with IntPredicate with IntUnaryOperator:
  super[IntUnaryOperator].andThen(this); super[IntPredicate].and(this)

class ConstructorTwoLinesPredicateFirst extends Parent with Marker with IntPredicate with IntUnaryOperator:
  super[IntPredicate].and(this)
  super[IntUnaryOperator].andThen(this)

class ConstructorTwoLinesOperatorFirst extends Parent with Marker with IntPredicate with IntUnaryOperator:
  super[IntUnaryOperator].andThen(this)
  super[IntPredicate].and(this)

class LambdaOneLinePredicateFirst extends Parent with Marker with IntPredicate with IntUnaryOperator:
  def both(): () => Unit = () =>
    super[IntPredicate].and(this); super[IntUnaryOperator].andThen(this)
    ()

class LambdaOneLineOperatorFirst extends Parent with Marker with IntPredicate with IntUnaryOperator:
  def both(): () => Unit = () =>
    super[IntUnaryOperator].andThen(this); super[IntPredicate].and(this)
    ()

class LambdaTwoLinesPredicateFirst extends Parent with Marker with IntPredicate with IntUnaryOperator:
  def both(): () => Unit = () =>
    super[IntPredicate].and(this)
    super[IntUnaryOperator].andThen(this)
    ()

class LambdaTwoLinesOperatorFirst extends Parent with Marker with IntPredicate with IntUnaryOperator:
  def both(): () => Unit = () =>
    super[IntUnaryOperator].andThen(this)
    super[IntPredicate].and(this)
    ()

abstract class CollectionParent extends java.util.AbstractCollection[String] with Predicate[String]:
  def size(): Int = 0
  def iterator(): java.util.Iterator[String] = java.util.Collections.emptyIterator[String]()
  def test(value: String): Boolean = false

// Both calls end at the same position. The inner call contributes its interface first.
class Nested extends CollectionParent with Marker with Collection[String] with Predicate[String]:
  def removeSelected(): Boolean =
    super[Collection].removeIf:
      super[Predicate].negate()

object Main:
  def main(args: Array[String]): Unit =
    def show(c: Class[?]): Unit = println(c.getSimpleName + ": " + c.getInterfaces.map(_.getSimpleName).mkString(","))
    show(classOf[MethodOneLinePredicateFirst])
    show(classOf[MethodOneLineOperatorFirst])
    show(classOf[MethodTwoLinesPredicateFirst])
    show(classOf[MethodTwoLinesOperatorFirst])
    show(classOf[ConstructorOneLinePredicateFirst])
    show(classOf[ConstructorOneLineOperatorFirst])
    show(classOf[ConstructorTwoLinesPredicateFirst])
    show(classOf[ConstructorTwoLinesOperatorFirst])
    show(classOf[LambdaOneLinePredicateFirst])
    show(classOf[LambdaOneLineOperatorFirst])
    show(classOf[LambdaTwoLinesPredicateFirst])
    show(classOf[LambdaTwoLinesOperatorFirst])
    show(classOf[Nested])
    println(new Nested().removeSelected())
