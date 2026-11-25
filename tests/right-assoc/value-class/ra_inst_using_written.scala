// ra_inst_using_written: a right-associative call's operands and their effects' order.
class K(val n: Int) {
  extension (x: Int) def +:(y: String)(using k: Int): String = y + x + k + n
}

object Main {
  var log = List.empty[String]
  def num(k: Int): Int = { log = log :+ ("n" + k); k }
  def note(s: String): String = { log = log :+ s; s }
  val field = 1
  val sfield = "a"
  given Int = 5
  val k = new K(7)
  def c_imp_imp_kept(): Unit = {
    log = Nil
    val res: String = k.+:(note("a"))(num(1))
    println("c_imp_imp_kept " + res + " " + log.mkString(","))
  }
  def c_imp_imp_any(): Unit = {
    log = Nil
    val res: Any = k.+:(note("a"))(num(1))
    println("c_imp_imp_any " + res + " " + log.mkString(","))
  }
  def c_imp_imp_dropped(): Unit = {
    log = Nil
    k.+:(note("a"))(num(1))
    println("c_imp_imp_dropped " + log.mkString(","))
  }
  def c_imp_lit_kept(): Unit = {
    log = Nil
    val res: String = k.+:("a")(num(1))
    println("c_imp_lit_kept " + res + " " + log.mkString(","))
  }
  def c_imp_lit_any(): Unit = {
    log = Nil
    val res: Any = k.+:("a")(num(1))
    println("c_imp_lit_any " + res + " " + log.mkString(","))
  }
  def c_imp_lit_dropped(): Unit = {
    log = Nil
    k.+:("a")(num(1))
    println("c_imp_lit_dropped " + log.mkString(","))
  }
  def c_imp_val_kept(): Unit = {
    log = Nil
    val rv = note("a")
    log = Nil
    val res: String = k.+:(rv)(num(1))
    println("c_imp_val_kept " + res + " " + log.mkString(","))
  }
  def c_imp_val_any(): Unit = {
    log = Nil
    val rv = note("a")
    log = Nil
    val res: Any = k.+:(rv)(num(1))
    println("c_imp_val_any " + res + " " + log.mkString(","))
  }
  def c_imp_val_dropped(): Unit = {
    log = Nil
    val rv = note("a")
    log = Nil
    k.+:(rv)(num(1))
    println("c_imp_val_dropped " + log.mkString(","))
  }
  def c_lit_imp_kept(): Unit = {
    log = Nil
    val res: String = k.+:(note("a"))(1)
    println("c_lit_imp_kept " + res + " " + log.mkString(","))
  }
  def c_lit_imp_any(): Unit = {
    log = Nil
    val res: Any = k.+:(note("a"))(1)
    println("c_lit_imp_any " + res + " " + log.mkString(","))
  }
  def c_lit_imp_dropped(): Unit = {
    log = Nil
    k.+:(note("a"))(1)
    println("c_lit_imp_dropped " + log.mkString(","))
  }
  def c_lit_lit_kept(): Unit = {
    log = Nil
    val res: String = k.+:("a")(1)
    println("c_lit_lit_kept " + res + " " + log.mkString(","))
  }
  def c_lit_lit_any(): Unit = {
    log = Nil
    val res: Any = k.+:("a")(1)
    println("c_lit_lit_any " + res + " " + log.mkString(","))
  }
  def c_lit_lit_dropped(): Unit = {
    log = Nil
    k.+:("a")(1)
    println("c_lit_lit_dropped " + log.mkString(","))
  }
  def c_val_val_kept(): Unit = {
    log = Nil
    val lv = num(1)
    val rv = note("a")
    log = Nil
    val res: String = k.+:(rv)(lv)
    println("c_val_val_kept " + res + " " + log.mkString(","))
  }
  def c_val_val_any(): Unit = {
    log = Nil
    val lv = num(1)
    val rv = note("a")
    log = Nil
    val res: Any = k.+:(rv)(lv)
    println("c_val_val_any " + res + " " + log.mkString(","))
  }
  def c_val_val_dropped(): Unit = {
    log = Nil
    val lv = num(1)
    val rv = note("a")
    log = Nil
    k.+:(rv)(lv)
    println("c_val_val_dropped " + log.mkString(","))
  }
  def c_fld_imp_kept(): Unit = {
    log = Nil
    val res: String = k.+:(note("a"))(field)
    println("c_fld_imp_kept " + res + " " + log.mkString(","))
  }
  def c_fld_imp_any(): Unit = {
    log = Nil
    val res: Any = k.+:(note("a"))(field)
    println("c_fld_imp_any " + res + " " + log.mkString(","))
  }
  def c_fld_imp_dropped(): Unit = {
    log = Nil
    k.+:(note("a"))(field)
    println("c_fld_imp_dropped " + log.mkString(","))
  }
  def c_fld_fld_kept(): Unit = {
    log = Nil
    val res: String = k.+:(sfield)(field)
    println("c_fld_fld_kept " + res + " " + log.mkString(","))
  }
  def c_fld_fld_any(): Unit = {
    log = Nil
    val res: Any = k.+:(sfield)(field)
    println("c_fld_fld_any " + res + " " + log.mkString(","))
  }
  def c_fld_fld_dropped(): Unit = {
    log = Nil
    k.+:(sfield)(field)
    println("c_fld_fld_dropped " + log.mkString(","))
  }
  def c_var_imp_kept(): Unit = {
    log = Nil
    var lvar = 1
    val res: String = k.+:(note("a"))(lvar)
    println("c_var_imp_kept " + res + " " + log.mkString(","))
  }
  def c_var_imp_any(): Unit = {
    log = Nil
    var lvar = 1
    val res: Any = k.+:(note("a"))(lvar)
    println("c_var_imp_any " + res + " " + log.mkString(","))
  }
  def c_var_imp_dropped(): Unit = {
    log = Nil
    var lvar = 1
    k.+:(note("a"))(lvar)
    println("c_var_imp_dropped " + log.mkString(","))
  }
  def c_mod_imp_kept(): Unit = {
    log = Nil
    val res: String = k.+:(note("a"))(MIc_mod_imp_kept.k)
    println("c_mod_imp_kept " + res + " " + log.mkString(","))
  }
  def c_mod_imp_any(): Unit = {
    log = Nil
    val res: Any = k.+:(note("a"))(MIc_mod_imp_any.k)
    println("c_mod_imp_any " + res + " " + log.mkString(","))
  }
  def c_mod_imp_dropped(): Unit = {
    log = Nil
    k.+:(note("a"))(MIc_mod_imp_dropped.k)
    println("c_mod_imp_dropped " + log.mkString(","))
  }
  def c_imp_mod_kept(): Unit = {
    log = Nil
    val res: String = k.+:(MSc_imp_mod_kept.s)(num(1))
    println("c_imp_mod_kept " + res + " " + log.mkString(","))
  }
  def c_imp_mod_any(): Unit = {
    log = Nil
    val res: Any = k.+:(MSc_imp_mod_any.s)(num(1))
    println("c_imp_mod_any " + res + " " + log.mkString(","))
  }
  def c_imp_mod_dropped(): Unit = {
    log = Nil
    k.+:(MSc_imp_mod_dropped.s)(num(1))
    println("c_imp_mod_dropped " + log.mkString(","))
  }
  def main(args: Array[String]): Unit = {
    try c_imp_imp_kept() catch { case e: Throwable => println("c_imp_imp_kept !" + e.getClass.getName) }
    try c_imp_imp_any() catch { case e: Throwable => println("c_imp_imp_any !" + e.getClass.getName) }
    try c_imp_imp_dropped() catch { case e: Throwable => println("c_imp_imp_dropped !" + e.getClass.getName) }
    try c_imp_lit_kept() catch { case e: Throwable => println("c_imp_lit_kept !" + e.getClass.getName) }
    try c_imp_lit_any() catch { case e: Throwable => println("c_imp_lit_any !" + e.getClass.getName) }
    try c_imp_lit_dropped() catch { case e: Throwable => println("c_imp_lit_dropped !" + e.getClass.getName) }
    try c_imp_val_kept() catch { case e: Throwable => println("c_imp_val_kept !" + e.getClass.getName) }
    try c_imp_val_any() catch { case e: Throwable => println("c_imp_val_any !" + e.getClass.getName) }
    try c_imp_val_dropped() catch { case e: Throwable => println("c_imp_val_dropped !" + e.getClass.getName) }
    try c_lit_imp_kept() catch { case e: Throwable => println("c_lit_imp_kept !" + e.getClass.getName) }
    try c_lit_imp_any() catch { case e: Throwable => println("c_lit_imp_any !" + e.getClass.getName) }
    try c_lit_imp_dropped() catch { case e: Throwable => println("c_lit_imp_dropped !" + e.getClass.getName) }
    try c_lit_lit_kept() catch { case e: Throwable => println("c_lit_lit_kept !" + e.getClass.getName) }
    try c_lit_lit_any() catch { case e: Throwable => println("c_lit_lit_any !" + e.getClass.getName) }
    try c_lit_lit_dropped() catch { case e: Throwable => println("c_lit_lit_dropped !" + e.getClass.getName) }
    try c_val_val_kept() catch { case e: Throwable => println("c_val_val_kept !" + e.getClass.getName) }
    try c_val_val_any() catch { case e: Throwable => println("c_val_val_any !" + e.getClass.getName) }
    try c_val_val_dropped() catch { case e: Throwable => println("c_val_val_dropped !" + e.getClass.getName) }
    try c_fld_imp_kept() catch { case e: Throwable => println("c_fld_imp_kept !" + e.getClass.getName) }
    try c_fld_imp_any() catch { case e: Throwable => println("c_fld_imp_any !" + e.getClass.getName) }
    try c_fld_imp_dropped() catch { case e: Throwable => println("c_fld_imp_dropped !" + e.getClass.getName) }
    try c_fld_fld_kept() catch { case e: Throwable => println("c_fld_fld_kept !" + e.getClass.getName) }
    try c_fld_fld_any() catch { case e: Throwable => println("c_fld_fld_any !" + e.getClass.getName) }
    try c_fld_fld_dropped() catch { case e: Throwable => println("c_fld_fld_dropped !" + e.getClass.getName) }
    try c_var_imp_kept() catch { case e: Throwable => println("c_var_imp_kept !" + e.getClass.getName) }
    try c_var_imp_any() catch { case e: Throwable => println("c_var_imp_any !" + e.getClass.getName) }
    try c_var_imp_dropped() catch { case e: Throwable => println("c_var_imp_dropped !" + e.getClass.getName) }
    try c_mod_imp_kept() catch { case e: Throwable => println("c_mod_imp_kept !" + e.getClass.getName) }
    try c_mod_imp_any() catch { case e: Throwable => println("c_mod_imp_any !" + e.getClass.getName) }
    try c_mod_imp_dropped() catch { case e: Throwable => println("c_mod_imp_dropped !" + e.getClass.getName) }
    try c_imp_mod_kept() catch { case e: Throwable => println("c_imp_mod_kept !" + e.getClass.getName) }
    try c_imp_mod_any() catch { case e: Throwable => println("c_imp_mod_any !" + e.getClass.getName) }
    try c_imp_mod_dropped() catch { case e: Throwable => println("c_imp_mod_dropped !" + e.getClass.getName) }
  }
}
object MIc_mod_imp_kept { Main.log = Main.log :+ "mi"; val k = 1 }
object MIc_mod_imp_any { Main.log = Main.log :+ "mi"; val k = 1 }
object MIc_mod_imp_dropped { Main.log = Main.log :+ "mi"; val k = 1 }
object MSc_imp_mod_kept { Main.log = Main.log :+ "ms"; val s = "a" }
object MSc_imp_mod_any { Main.log = Main.log :+ "ms"; val s = "a" }
object MSc_imp_mod_dropped { Main.log = Main.log :+ "ms"; val s = "a" }
