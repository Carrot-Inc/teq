package tsa

trait Greeter:
  def greet(name: String): String = s"hello $name"

trait Loud extends Greeter:
  override def greet(name: String): String = super.greet(name).toUpperCase

trait Polite extends Greeter:
  override def greet(name: String): String = super.greet(name) + ", please"
