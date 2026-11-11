// Eight definitions of one file expand a transparent call each while they are typed: dotty numbers the
// proxies in the order it types the file, m$proxy1 to m$proxy8, whichever worker types what.
object A:
  def arg(): Int = 1
  def text: String = lib.Names.bound(arg())
object B:
  def arg(): Int = 2
  def text: String = lib.Names.bound(arg())
object C:
  def arg(): Int = 3
  def text: String = lib.Names.bound(arg())
object D:
  def arg(): Int = 4
  def text: String = lib.Names.bound(arg())
object E:
  def arg(): Int = 5
  def text: String = lib.Names.bound(arg())
object F:
  def arg(): Int = 6
  def text: String = lib.Names.bound(arg())
object G:
  def arg(): Int = 7
  def text: String = lib.Names.bound(arg())
object H:
  def arg(): Int = 8
  def text: String = lib.Names.bound(arg())

@main def run(): Unit = println(List(A.text, B.text, C.text, D.text, E.text, F.text, G.text, H.text).mkString(","))
