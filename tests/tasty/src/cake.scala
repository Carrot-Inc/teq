package fix.cake

trait Defs:
  type Box[A]
  val Box: BoxModule
  trait BoxModule:
    def wrap[A](a: A): Box[A]
  def unwrap[A](b: Box[A]): A

trait DefsPlatform extends Defs:
  final override type Box[A] = List[A]
  object Box extends BoxModule:
    def wrap[A](a: A): List[A] = List(a)
  def unwrap[A](b: List[A]): A = b.head

final class Cake extends DefsPlatform:
  def boxed(n: Int): Box[Int] = Box.wrap(n)
  def twice(b: Box[Int]): Box[Int] = b ++ b

class Outer(val tag: String):
  trait Named:
    def name: String
  object Named:
    def make(n: String): Named = new Named:
      def name: String = n + tag
    def plain(n: String): String = n + tag
  object Lone:
    def get: String = tag
  object Peel:
    def unapply(s: String): Option[String] =
      if s.startsWith("(") then s.drop(1) match
        case Peel(r) => Some(r)
        case r => Some(r)
      else Some(s + tag)
  def run(n: String): String = Named.plain(n) + " " + Lone.get + " " + Named.make(n).name
  def peel(s: String): String = s match
    case Peel(r) => r

trait Module:
  def tag: String
  val Named: NamedModule
  trait NamedModule:
    def make(n: String): String
    def hinted(n: String, loud: Boolean = false): String = if loud then make(n).toUpperCase else make(n)
  val Helper: HelperModule
  trait HelperModule:
    def help(n: String): String = Named.hinted(n, loud = true) + "?"
    def box(n: String): Boxed = new Boxed(n)
  class Boxed(val n: String):
    def show: String = n + tag
  def useIt(n: String): String = Helper.help(n) + " " + Named.make(n) + " " + Helper.box(n).show

final class ModuleImpl(val tag: String) extends Module:
  object Named extends NamedModule:
    def make(n: String): String = n + tag
    object Inner:
      def get: String = "inner" + tag
  object Helper extends HelperModule
  def inner: String = Named.Inner.get

trait ModulePlatform extends Module:
  object Named extends NamedModule:
    def make(n: String): String = n + tag + tag
  object Helper extends HelperModule

final class ModulePlatformImpl extends ModulePlatform:
  def tag: String = "~"

trait SelfDefs:
  type E[A]

trait SelfUses:
  this: SelfDefs =>
  def f[A](e: E[A]): E[A] = e
  trait Helper:
    def wrap[A](e: E[A]): E[A] = f(e)
  val Helper: Helper

trait SelfOther:
  this: SelfDefs & SelfUses =>
  def g[A](e: E[A]): E[A] = f(e)
  def h[A](e: E[A]): E[A] = Helper.wrap(e)
  def viaParam(u: SelfDefs & SelfUses)(e: u.E[Int]): u.E[Int] = u.Helper.wrap(e)

final class SelfImpl extends SelfDefs, SelfUses, SelfOther:
  type E[A] = List[A]
  object Helper extends Helper

trait PromiseDefs:
  type Name

trait PromiseDefsPlatform extends PromiseDefs:
  final override type Name = String

trait Promises:
  this: PromiseDefs =>
  final class Promise[A](val name: Name)
  protected val Promise: PromiseModule
  protected trait PromiseModule:
    this: Promise.type =>
    final def promise[A](hint: Name): Promise[A] = new Promise[A](fresh(hint))
    protected def fresh(hint: Name): Name
  def run(hint: Name): Name = Promise.promise[Int](hint).name

trait PromisesPlatform extends Promises:
  this: PromiseDefsPlatform =>
  protected object Promise extends PromiseModule:
    def fresh(hint: String): String = hint + "'"

final class PromisesImpl extends PromiseDefsPlatform, PromisesPlatform
