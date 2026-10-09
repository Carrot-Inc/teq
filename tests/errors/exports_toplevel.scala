// expect: cannot export nothing: it is not a member of Source
// expect: cannot resolve export: nowhere not found

package demo

export demo.Source.{one, nothing}
export nowhere.Thing

object Source:
  def one: Int = 1

@main def run(): Unit = println("unused")
