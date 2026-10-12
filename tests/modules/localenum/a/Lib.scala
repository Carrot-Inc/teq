package lea

// Local enums of upstream bodies: the companion a local object of each run, the values its own;
// an inline body's enum is the stored body's, copied at each expansion.
object Lib:
  def value(n: Int): AnyRef =
    enum E:
      case A, B
      def get = n
    E.A
  def read(n: Int): Int =
    enum E:
      case A
      def get = n
    E.A.get
  // scalac warns that the value cases' classes are duplicated at each expansion (E197).
  @annotation.nowarn("msg=New anonymous class definition")
  inline def ordinal(): Int =
    enum E:
      case A, B
    E.B.ordinal
