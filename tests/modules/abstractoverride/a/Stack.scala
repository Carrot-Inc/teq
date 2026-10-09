package aoa

trait Base:
  def f: Int
  def name: String

class Impl extends Base:
  def f: Int = 1
  def name: String = "impl"

trait Plus extends Base:
  abstract override def f: Int = super.f + 1

trait Loud extends Base:
  abstract override def name: String = super.name.toUpperCase
