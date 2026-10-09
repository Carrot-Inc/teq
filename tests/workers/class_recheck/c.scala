package p

object Target extends Base:
  inline def local: Unit = noop
  local
  def value: Int = 42

class Maker extends Factory:
  def make(): Base = Target

inline implicit def selected: Factory = new Maker
