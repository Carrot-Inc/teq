package liba
// version 2: the declarations stand one line lower than in version 1
class A:
  def hello: Int = 10
  def base: Int = 20
  def added: Int = 30
object A:
  def make: A = new A
