// A quote that keeps an inline call for its expansion (`'{ count[et & Tuple] }` of circe's
// derivation): the types its type arguments name are the quote's, so `et`, bound by the
// pattern, is substituted when the quote runs.

import scala.quoted.*
import scala.deriving.Mirror
import scala.compiletime.*

inline def count[T <: Tuple]: Int = inline erasedValue[T] match
  case _: EmptyTuple => 0
  case _: (h *: ts) => 1 + count[ts]

inline def fields[A](using inline m: Mirror.Of[A]): Int = ${ impl[A]('m) }
def impl[A: Type](m: Expr[Mirror.Of[A]])(using Quotes): Expr[Int] = m match
  case '{ $m: Mirror.ProductOf[A] { type MirroredElemTypes = et } } => '{ count[et & Tuple] }
  case _ => '{ -1 }

inline def count2[T]: Int = inline erasedValue[T] match
  case _: EmptyTuple => 0
  case _: (h *: ts) => 1 + count2[ts]

inline def fields2[A](using inline m: Mirror.Of[A]): Int = ${ impl2[A]('m) }
def impl2[A: Type](m: Expr[Mirror.Of[A]])(using Quotes): Expr[Int] = m match
  case '{ $m: Mirror.ProductOf[A] { type MirroredElemTypes = et } } => '{ count2[et] }
  case _ => '{ -1 }

inline def countL[T]: Int = inline erasedValue[T] match
  case _: List[t] => count2[t]
  case _ => -2

inline def fields3[A](using inline m: Mirror.Of[A]): Int = ${ impl3[A]('m) }
def impl3[A: Type](m: Expr[Mirror.Of[A]])(using Quotes): Expr[Int] = m match
  case '{ $m: Mirror.ProductOf[A] { type MirroredElemTypes = et } } => '{ countL[List[et]] }
  case _ => '{ -1 }

inline def fields4[A](using inline m: Mirror.Of[A]): Int = ${ impl4[A]('m) }
def impl4[A: Type](m: Expr[Mirror.Of[A]])(using Quotes): Expr[Int] = m match
  case '{ $m: Mirror.ProductOf[A] { type MirroredElemTypes = et } } => '{ count2[et & Product] }
  case _ => '{ -1 }
