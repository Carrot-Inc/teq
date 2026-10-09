package p

import scala.annotation.unchecked.uncheckedVariance

class Box9[-A]:
  def last: Option[A @uncheckedVariance] = None
  def all: List[A @uncheckedVariance] = Nil

object Use9:
  def size: Int = Box10[Int]().all.size + Box10[String]().last.size
