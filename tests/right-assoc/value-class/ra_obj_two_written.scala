// ra_obj_two_written: a right-associative call's operands and their effects' order.
object O {
  extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
}

object Main {
  var log = List.empty[String]
  def num(k: Int): Int = { log = log :+ ("n" + k); k }
  def note(s: String): String = { log = log :+ s; s }
  val field = 1
  val sfield = "a"
  def c_imp_imp_kept(): Unit = {
    log = Nil
    val res: String = (O.+:(note("a"))(num(1)))(num(2))
    println("c_imp_imp_kept " + res + " " + log.mkString(","))
  }
  def c_imp_imp_any(): Unit = {
    log = Nil
    val res: Any = (O.+:(note("a"))(num(1)))(num(2))
    println("c_imp_imp_any " + res + " " + log.mkString(","))
  }
  def c_imp_imp_dropped(): Unit = {
    log = Nil
    (O.+:(note("a"))(num(1)))(num(2))
    println("c_imp_imp_dropped " + log.mkString(","))
  }
  def c_imp_lit_kept(): Unit = {
    log = Nil
    val res: String = (O.+:("a")(num(1)))(num(2))
    println("c_imp_lit_kept " + res + " " + log.mkString(","))
  }
  def c_imp_lit_any(): Unit = {
    log = Nil
    val res: Any = (O.+:("a")(num(1)))(num(2))
    println("c_imp_lit_any " + res + " " + log.mkString(","))
  }
  def c_imp_lit_dropped(): Unit = {
    log = Nil
    (O.+:("a")(num(1)))(num(2))
    println("c_imp_lit_dropped " + log.mkString(","))
  }
  def c_imp_val_kept(): Unit = {
    log = Nil
    val rv = note("a")
    log = Nil
    val res: String = (O.+:(rv)(num(1)))(num(2))
    println("c_imp_val_kept " + res + " " + log.mkString(","))
  }
  def c_imp_val_any(): Unit = {
    log = Nil
    val rv = note("a")
    log = Nil
    val res: Any = (O.+:(rv)(num(1)))(num(2))
    println("c_imp_val_any " + res + " " + log.mkString(","))
  }
  def c_imp_val_dropped(): Unit = {
    log = Nil
    val rv = note("a")
    log = Nil
    (O.+:(rv)(num(1)))(num(2))
    println("c_imp_val_dropped " + log.mkString(","))
  }
  def c_lit_imp_kept(): Unit = {
    log = Nil
    val res: String = (O.+:(note("a"))(1))(num(2))
    println("c_lit_imp_kept " + res + " " + log.mkString(","))
  }
  def c_lit_imp_any(): Unit = {
    log = Nil
    val res: Any = (O.+:(note("a"))(1))(num(2))
    println("c_lit_imp_any " + res + " " + log.mkString(","))
  }
  def c_lit_imp_dropped(): Unit = {
    log = Nil
    (O.+:(note("a"))(1))(num(2))
    println("c_lit_imp_dropped " + log.mkString(","))
  }
  def c_lit_lit_kept(): Unit = {
    log = Nil
    val res: String = (O.+:("a")(1))(num(2))
    println("c_lit_lit_kept " + res + " " + log.mkString(","))
  }
  def c_lit_lit_any(): Unit = {
    log = Nil
    val res: Any = (O.+:("a")(1))(num(2))
    println("c_lit_lit_any " + res + " " + log.mkString(","))
  }
  def c_lit_lit_dropped(): Unit = {
    log = Nil
    (O.+:("a")(1))(num(2))
    println("c_lit_lit_dropped " + log.mkString(","))
  }
  def c_val_val_kept(): Unit = {
    log = Nil
    val lv = num(1)
    val rv = note("a")
    log = Nil
    val res: String = (O.+:(rv)(lv))(num(2))
    println("c_val_val_kept " + res + " " + log.mkString(","))
  }
  def c_val_val_any(): Unit = {
    log = Nil
    val lv = num(1)
    val rv = note("a")
    log = Nil
    val res: Any = (O.+:(rv)(lv))(num(2))
    println("c_val_val_any " + res + " " + log.mkString(","))
  }
  def c_val_val_dropped(): Unit = {
    log = Nil
    val lv = num(1)
    val rv = note("a")
    log = Nil
    (O.+:(rv)(lv))(num(2))
    println("c_val_val_dropped " + log.mkString(","))
  }
  def c_fld_imp_kept(): Unit = {
    log = Nil
    val res: String = (O.+:(note("a"))(field))(num(2))
    println("c_fld_imp_kept " + res + " " + log.mkString(","))
  }
  def c_fld_imp_any(): Unit = {
    log = Nil
    val res: Any = (O.+:(note("a"))(field))(num(2))
    println("c_fld_imp_any " + res + " " + log.mkString(","))
  }
  def c_fld_imp_dropped(): Unit = {
    log = Nil
    (O.+:(note("a"))(field))(num(2))
    println("c_fld_imp_dropped " + log.mkString(","))
  }
  def c_fld_fld_kept(): Unit = {
    log = Nil
    val res: String = (O.+:(sfield)(field))(num(2))
    println("c_fld_fld_kept " + res + " " + log.mkString(","))
  }
  def c_fld_fld_any(): Unit = {
    log = Nil
    val res: Any = (O.+:(sfield)(field))(num(2))
    println("c_fld_fld_any " + res + " " + log.mkString(","))
  }
  def c_fld_fld_dropped(): Unit = {
    log = Nil
    (O.+:(sfield)(field))(num(2))
    println("c_fld_fld_dropped " + log.mkString(","))
  }
  def c_var_imp_kept(): Unit = {
    log = Nil
    var lvar = 1
    val res: String = (O.+:(note("a"))(lvar))(num(2))
    println("c_var_imp_kept " + res + " " + log.mkString(","))
  }
  def c_var_imp_any(): Unit = {
    log = Nil
    var lvar = 1
    val res: Any = (O.+:(note("a"))(lvar))(num(2))
    println("c_var_imp_any " + res + " " + log.mkString(","))
  }
  def c_var_imp_dropped(): Unit = {
    log = Nil
    var lvar = 1
    (O.+:(note("a"))(lvar))(num(2))
    println("c_var_imp_dropped " + log.mkString(","))
  }
  def c_mod_imp_kept(): Unit = {
    log = Nil
    val res: String = (O.+:(note("a"))(MIc_mod_imp_kept.k))(num(2))
    println("c_mod_imp_kept " + res + " " + log.mkString(","))
  }
  def c_mod_imp_any(): Unit = {
    log = Nil
    val res: Any = (O.+:(note("a"))(MIc_mod_imp_any.k))(num(2))
    println("c_mod_imp_any " + res + " " + log.mkString(","))
  }
  def c_mod_imp_dropped(): Unit = {
    log = Nil
    (O.+:(note("a"))(MIc_mod_imp_dropped.k))(num(2))
    println("c_mod_imp_dropped " + log.mkString(","))
  }
  def c_imp_mod_kept(): Unit = {
    log = Nil
    val res: String = (O.+:(MSc_imp_mod_kept.s)(num(1)))(num(2))
    println("c_imp_mod_kept " + res + " " + log.mkString(","))
  }
  def c_imp_mod_any(): Unit = {
    log = Nil
    val res: Any = (O.+:(MSc_imp_mod_any.s)(num(1)))(num(2))
    println("c_imp_mod_any " + res + " " + log.mkString(","))
  }
  def c_imp_mod_dropped(): Unit = {
    log = Nil
    (O.+:(MSc_imp_mod_dropped.s)(num(1)))(num(2))
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
