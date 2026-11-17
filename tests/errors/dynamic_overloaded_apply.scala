import scala.language.dynamics
class Two extends Dynamic:
  def applyDynamic(name: String)(a: Int): String = name
  def applyDynamic(name: String)(a: Int, b: Int): String = name
class Generic extends Dynamic:
  def applyDynamic[T](name: String)(a: T): String = name
  def applyDynamic[T, U](name: String)(a: T, b: U): String = name
class Base extends Dynamic:
  def applyDynamic(name: String)(a: Int): String = name
class Adds extends Base:
  def applyDynamic(name: String)(a: String): String = name
class Plain:
  def applyDynamic(name: String)(a: Int): String = name
  def applyDynamic(name: String)(a: Int, b: Int): String = name
// `applyDynamic` overloaded in a class that derives from `scala.Dynamic` (`RefChecks.checkOverloadedRestrictions`):
// refused at the last alternative the class declares, also where the alternatives differ in their type parameters'
// number (scalac 3.8.4 refuses that too) and where one of two is inherited; a class that does not derive from it is free.
// expect: 4:7: error: implementation restriction: applyDynamic cannot be overloaded except by methods with different numbers of type parameters
// expect: 7:7: error: implementation restriction: applyDynamic cannot be overloaded
// expect: 11:7: error: implementation restriction: applyDynamic cannot be overloaded
// expect: 3 errors found
// absent: 14:7: error:
