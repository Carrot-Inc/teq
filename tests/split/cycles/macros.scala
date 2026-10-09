// Macros whose runs leave cycles of references behind in the interpreter, one shape each: what
// a session's memory has to come back from (tests/split-watch.sh, docs/SPEED.md "A session's
// memory").
package cycles

import scala.quoted.*

class Node(val payload: Array[Any]):
  var parent: Node = null
  var children: List[Node] = Nil

class Holder(val payload: Array[Any]):
  val back: Keeper = new Keeper(this)

class Keeper(val holder: Holder)

class Self(val payload: Array[Any]):
  val me: () => Self = () => this

object Macros:
  /// A closure over a nested scope, kept by the scope around it: outer -> closure -> inner,
  /// whose parent is outer.
  inline def nested: Int = ${ nestedImpl }
  def nestedImpl(using Quotes): Expr[Int] =
    var sum = 0
    var i = 0
    while i < 100 do
      val kept = new Array[Any](1000)
      val f = { val n = 7; () => n + kept.length }
      sum += f()
      i += 1
    Expr(sum)

  /// A frame that a closure captures and that keeps the closure.
  inline def captured: Int = ${ capturedImpl }
  def capturedImpl(using Quotes): Expr[Int] =
    def once(i: Int): Int =
      val kept = new Array[Any](1000)
      val f = () => kept.length + i
      f()
    var sum = 0
    var i = 0
    while i < 100 do
      sum += once(i)
      i += 1
    Expr(sum)

  /// An array that holds itself, with no closure anywhere.
  inline def array: Int = ${ arrayImpl }
  def arrayImpl(using Quotes): Expr[Int] =
    var i = 0
    while i < 100 do
      val a = new Array[Any](1000)
      a(0) = a
      i += 1
    Expr(42)

  /// A tree with links to the parents, a field written after the node was made.
  inline def tree: Int = ${ treeImpl }
  def treeImpl(using Quotes): Expr[Int] =
    var i = 0
    while i < 100 do
      val root = new Node(new Array[Any](1000))
      val leaf = new Node(new Array[Any](10))
      leaf.parent = root
      root.children = leaf :: root.children
      i += 1
    Expr(i)

  /// Two objects that name each other from their constructors.
  inline def built: Int = ${ builtImpl }
  def builtImpl(using Quotes): Expr[Int] =
    var i = 0
    while i < 100 do
      val h = new Holder(new Array[Any](1000))
      if h.back.holder ne h then i += 1000
      i += 1
    Expr(i)

  /// An object kept by a closure of its own field.
  inline def self: Int = ${ selfImpl }
  def selfImpl(using Quotes): Expr[Int] =
    var i = 0
    while i < 100 do
      val s = new Self(new Array[Any](1000))
      if s.me() ne s then i += 1000
      i += 1
    Expr(i)
