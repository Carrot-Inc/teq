// A member whose body is typed where it is first asked for, and an inline call's arguments.
trait Demanded extends Ground:
  def one = two + super.b()
  def two = super.a()

trait Dropped extends Ground:
  def out: Int = { ignore(super.b().max(super.c())); super.a() }

trait Moved extends Ground:
  def out: Int = swapped(super.a(), super.b())
