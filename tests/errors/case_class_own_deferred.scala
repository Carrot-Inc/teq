// A concrete case class's own deferred declaration of a member the rule gives is an obligation
// of its own (dotty's `RefChecks.checkNoAbstractDecls`), which the synthesized member, entered
// beside it, does not discharge; a trait's or an abstract superclass's redeclaration it does.
case class Unfinished(x: Int):
  override def toString: String
// expect: class Unfinished needs to be abstract, since there is a deferred declaration of method toString of type => String which is not implemented in a subclass

case class Twice(x: Int):
  override def toString: String
  override def hashCode: Int
// expect: class Twice needs to be abstract, since:
// expect: there is a deferred declaration of method toString of type => String which is not implemented in a subclass
// expect: there is a deferred declaration of method hashCode of type => Int which is not implemented in a subclass

trait T:
  override def toString: String
case class FromTrait(x: Int) extends T
// absent: class FromTrait needs to be abstract

abstract class A:
  override def toString: String
case class FromSuper(x: Int) extends A
// absent: class FromSuper needs to be abstract

@main def run(): Unit = println(FromTrait(1).toString + FromSuper(2))
