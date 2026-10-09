package p

import scala.quoted.*

trait Has:
  def n: Int

trait AF:
  def make(): Has

trait BF:
  def make(): Has

inline def readA: Int = ${ implA }
inline def readB: Int = ${ implB }
def implA(using Quotes): Expr[Int] = Expr(summon[AF].make().n)
def implB(using Quotes): Expr[Int] = Expr(summon[BF].make().n)
