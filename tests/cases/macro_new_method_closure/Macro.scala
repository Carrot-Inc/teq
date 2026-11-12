// The block of a new method's definition and the closure over it, which is a lambda to dotty (`Lambda.unapply`)
// and how tapir's form codec makes its two functions: of one parameter, of none, of two with a captured
// argument, and expanded several times in one method and under one owner.
import scala.quoted.*

inline def addOne: Int => Int = ${ addOneImpl }

def addOneImpl(using q: Quotes): Expr[Int => Int] =
  import q.reflect.*
  val tpe = MethodType(List("t"))(_ => List(TypeRepr.of[Int]), _ => TypeRepr.of[Int])
  val sym = Symbol.newMethod(Symbol.spliceOwner, "added", tpe)
  val defined = DefDef(sym, {
    case List(List(t: Term)) => Some('{ ${ t.asExprOf[Int] } + 1 }.asTerm.changeOwner(sym))
    case other => report.errorAndAbort("the parameters the body was given: " + other)
  })
  Block(List(defined), Closure(Ref(sym), None)).asExprOf[Int => Int]

inline def thunk: () => Int = ${ thunkImpl }

def thunkImpl(using q: Quotes): Expr[() => Int] =
  import q.reflect.*
  val tpe = MethodType(Nil)(_ => Nil, _ => TypeRepr.of[Int])
  val sym = Symbol.newMethod(Symbol.spliceOwner, "answer", tpe)
  val defined = DefDef(sym, {
    case List(Nil) => Some('{ 42 }.asTerm.changeOwner(sym))
    case other => report.errorAndAbort("the parameters the body was given: " + other)
  })
  Block(List(defined), Closure(Ref(sym), None)).asExprOf[() => Int]

inline def joiner(inline k: Int): (Int, String) => String = ${ joinerImpl('k) }

def joinerImpl(k: Expr[Int])(using q: Quotes): Expr[(Int, String) => String] =
  import q.reflect.*
  val tpe = MethodType(List("n", "s"))(_ => List(TypeRepr.of[Int], TypeRepr.of[String]), _ => TypeRepr.of[String])
  val sym = Symbol.newMethod(Symbol.spliceOwner, "join", tpe)
  val defined = DefDef(sym, {
    case List(List(n: Term, s: Term)) => Some('{ ${ s.asExprOf[String] } + (${ n.asExprOf[Int] } + $k) }.asTerm.changeOwner(sym))
    case other => report.errorAndAbort("the parameters the body was given: " + other)
  })
  Block(List(defined), Closure(Ref(sym), None)).asExprOf[(Int, String) => String]
