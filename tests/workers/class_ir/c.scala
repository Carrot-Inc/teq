package p

class Concrete extends Base:
  val value: Int = 42

class Maker extends Factory:
  def make(): Base = new Concrete

inline implicit def selected: Factory = new Maker
