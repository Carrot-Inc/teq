package p

import scala.annotation.unchecked.uncheckedVariance

class Box0[-A]:
  def last: Option[A @uncheckedVariance] = None
  def all: List[A @uncheckedVariance] = Nil

object Use0:
  def size: Int = Box1[Int]().all.size + Box1[String]().last.size
