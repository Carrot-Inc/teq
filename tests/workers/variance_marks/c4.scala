package p

import scala.annotation.unchecked.uncheckedVariance

class Box4[-A]:
  def last: Option[A @uncheckedVariance] = None
  def all: List[A @uncheckedVariance] = Nil

object Use4:
  def size: Int = Box5[Int]().all.size + Box5[String]().last.size
