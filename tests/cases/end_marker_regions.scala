object Before:
  def one = 1
  end one
end Before
@main def endRegions(): Unit =
  println(nested.After.two)
end endRegions
package nested:
  object After:
    def two = 2
    end two
  end After
  trait Base
  val value =
    new Base:
      def member = 3
    end new
end nested
