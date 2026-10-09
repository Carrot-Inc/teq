import scala.quoted.*

// A file with quoted code, typed before the fork: its run hashes S on the first worker's heap
// before any run reads S.x.
object U0:
  val h0: Int = h
  def quoted(using Quotes): Expr[Int] = '{ 1 }
