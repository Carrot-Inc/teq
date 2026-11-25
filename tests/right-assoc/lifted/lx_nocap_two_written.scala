// lx_nocap_two_written: a local trait's right-associative extension member, its operands' effects' order.
object Main {
  var log = List.empty[String]
  def num(k: Int): Int = { log = log :+ ("n" + k); k }
  def note(s: String): String = { log = log :+ s; s }
  val field = 1
  val sfield = "a"
  def c_imp_imp_kept(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    val res: String = (+:(note("a"))(num(1)))(num(2))
    println("c_imp_imp_kept " + res + " " + log.mkString(","))
  }
  def c_imp_imp_any(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    val res: Any = (+:(note("a"))(num(1)))(num(2))
    println("c_imp_imp_any " + res + " " + log.mkString(","))
  }
  def c_imp_imp_dropped(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    (+:(note("a"))(num(1)))(num(2))
    println("c_imp_imp_dropped " + log.mkString(","))
  }
  def c_imp_lit_kept(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    val res: String = (+:("a")(num(1)))(num(2))
    println("c_imp_lit_kept " + res + " " + log.mkString(","))
  }
  def c_imp_lit_any(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    val res: Any = (+:("a")(num(1)))(num(2))
    println("c_imp_lit_any " + res + " " + log.mkString(","))
  }
  def c_imp_lit_dropped(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    (+:("a")(num(1)))(num(2))
    println("c_imp_lit_dropped " + log.mkString(","))
  }
  def c_imp_val_kept(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    val rv = note("a")
    log = Nil
    val res: String = (+:(rv)(num(1)))(num(2))
    println("c_imp_val_kept " + res + " " + log.mkString(","))
  }
  def c_imp_val_any(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    val rv = note("a")
    log = Nil
    val res: Any = (+:(rv)(num(1)))(num(2))
    println("c_imp_val_any " + res + " " + log.mkString(","))
  }
  def c_imp_val_dropped(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    val rv = note("a")
    log = Nil
    (+:(rv)(num(1)))(num(2))
    println("c_imp_val_dropped " + log.mkString(","))
  }
  def c_lit_imp_kept(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    val res: String = (+:(note("a"))(1))(num(2))
    println("c_lit_imp_kept " + res + " " + log.mkString(","))
  }
  def c_lit_imp_any(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    val res: Any = (+:(note("a"))(1))(num(2))
    println("c_lit_imp_any " + res + " " + log.mkString(","))
  }
  def c_lit_imp_dropped(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    (+:(note("a"))(1))(num(2))
    println("c_lit_imp_dropped " + log.mkString(","))
  }
  def c_lit_lit_kept(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    val res: String = (+:("a")(1))(num(2))
    println("c_lit_lit_kept " + res + " " + log.mkString(","))
  }
  def c_lit_lit_any(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    val res: Any = (+:("a")(1))(num(2))
    println("c_lit_lit_any " + res + " " + log.mkString(","))
  }
  def c_lit_lit_dropped(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    (+:("a")(1))(num(2))
    println("c_lit_lit_dropped " + log.mkString(","))
  }
  def c_val_val_kept(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    val lv = num(1)
    val rv = note("a")
    log = Nil
    val res: String = (+:(rv)(lv))(num(2))
    println("c_val_val_kept " + res + " " + log.mkString(","))
  }
  def c_val_val_any(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    val lv = num(1)
    val rv = note("a")
    log = Nil
    val res: Any = (+:(rv)(lv))(num(2))
    println("c_val_val_any " + res + " " + log.mkString(","))
  }
  def c_val_val_dropped(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    val lv = num(1)
    val rv = note("a")
    log = Nil
    (+:(rv)(lv))(num(2))
    println("c_val_val_dropped " + log.mkString(","))
  }
  def c_fld_imp_kept(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    val res: String = (+:(note("a"))(field))(num(2))
    println("c_fld_imp_kept " + res + " " + log.mkString(","))
  }
  def c_fld_imp_any(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    val res: Any = (+:(note("a"))(field))(num(2))
    println("c_fld_imp_any " + res + " " + log.mkString(","))
  }
  def c_fld_imp_dropped(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    (+:(note("a"))(field))(num(2))
    println("c_fld_imp_dropped " + log.mkString(","))
  }
  def c_fld_fld_kept(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    val res: String = (+:(sfield)(field))(num(2))
    println("c_fld_fld_kept " + res + " " + log.mkString(","))
  }
  def c_fld_fld_any(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    val res: Any = (+:(sfield)(field))(num(2))
    println("c_fld_fld_any " + res + " " + log.mkString(","))
  }
  def c_fld_fld_dropped(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    (+:(sfield)(field))(num(2))
    println("c_fld_fld_dropped " + log.mkString(","))
  }
  def c_var_imp_kept(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    var lvar = 1
    log = Nil
    val res: String = (+:(note("a"))(lvar))(num(2))
    println("c_var_imp_kept " + res + " " + log.mkString(","))
  }
  def c_var_imp_any(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    var lvar = 1
    log = Nil
    val res: Any = (+:(note("a"))(lvar))(num(2))
    println("c_var_imp_any " + res + " " + log.mkString(","))
  }
  def c_var_imp_dropped(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    var lvar = 1
    log = Nil
    (+:(note("a"))(lvar))(num(2))
    println("c_var_imp_dropped " + log.mkString(","))
  }
  object O_obj_imp_kept { log = log :+ "I"; val v = 3; val s = "o" }
  object P_obj_imp_kept { log = log :+ "J"; val v = 4; val s = "p" }
  def c_obj_imp_kept(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    val res: String = (+:(note("a"))(O_obj_imp_kept.v))(num(2))
    println("c_obj_imp_kept " + res + " " + log.mkString(","))
  }
  object O_obj_imp_any { log = log :+ "I"; val v = 3; val s = "o" }
  object P_obj_imp_any { log = log :+ "J"; val v = 4; val s = "p" }
  def c_obj_imp_any(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    val res: Any = (+:(note("a"))(O_obj_imp_any.v))(num(2))
    println("c_obj_imp_any " + res + " " + log.mkString(","))
  }
  object O_obj_imp_dropped { log = log :+ "I"; val v = 3; val s = "o" }
  object P_obj_imp_dropped { log = log :+ "J"; val v = 4; val s = "p" }
  def c_obj_imp_dropped(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    (+:(note("a"))(O_obj_imp_dropped.v))(num(2))
    println("c_obj_imp_dropped " + log.mkString(","))
  }
  object O_imp_obj_kept { log = log :+ "I"; val v = 3; val s = "o" }
  object P_imp_obj_kept { log = log :+ "J"; val v = 4; val s = "p" }
  def c_imp_obj_kept(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    val res: String = (+:(O_imp_obj_kept.s)(num(1)))(num(2))
    println("c_imp_obj_kept " + res + " " + log.mkString(","))
  }
  object O_imp_obj_any { log = log :+ "I"; val v = 3; val s = "o" }
  object P_imp_obj_any { log = log :+ "J"; val v = 4; val s = "p" }
  def c_imp_obj_any(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    val res: Any = (+:(O_imp_obj_any.s)(num(1)))(num(2))
    println("c_imp_obj_any " + res + " " + log.mkString(","))
  }
  object O_imp_obj_dropped { log = log :+ "I"; val v = 3; val s = "o" }
  object P_imp_obj_dropped { log = log :+ "J"; val v = 4; val s = "p" }
  def c_imp_obj_dropped(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    (+:(O_imp_obj_dropped.s)(num(1)))(num(2))
    println("c_imp_obj_dropped " + log.mkString(","))
  }
  object O_obj_lit_kept { log = log :+ "I"; val v = 3; val s = "o" }
  object P_obj_lit_kept { log = log :+ "J"; val v = 4; val s = "p" }
  def c_obj_lit_kept(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    val res: String = (+:("a")(O_obj_lit_kept.v))(num(2))
    println("c_obj_lit_kept " + res + " " + log.mkString(","))
  }
  object O_obj_lit_any { log = log :+ "I"; val v = 3; val s = "o" }
  object P_obj_lit_any { log = log :+ "J"; val v = 4; val s = "p" }
  def c_obj_lit_any(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    val res: Any = (+:("a")(O_obj_lit_any.v))(num(2))
    println("c_obj_lit_any " + res + " " + log.mkString(","))
  }
  object O_obj_lit_dropped { log = log :+ "I"; val v = 3; val s = "o" }
  object P_obj_lit_dropped { log = log :+ "J"; val v = 4; val s = "p" }
  def c_obj_lit_dropped(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    (+:("a")(O_obj_lit_dropped.v))(num(2))
    println("c_obj_lit_dropped " + log.mkString(","))
  }
  object O_obj_obj_kept { log = log :+ "I"; val v = 3; val s = "o" }
  object P_obj_obj_kept { log = log :+ "J"; val v = 4; val s = "p" }
  def c_obj_obj_kept(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    val res: String = (+:(P_obj_obj_kept.s)(O_obj_obj_kept.v))(num(2))
    println("c_obj_obj_kept " + res + " " + log.mkString(","))
  }
  object O_obj_obj_any { log = log :+ "I"; val v = 3; val s = "o" }
  object P_obj_obj_any { log = log :+ "J"; val v = 4; val s = "p" }
  def c_obj_obj_any(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    val res: Any = (+:(P_obj_obj_any.s)(O_obj_obj_any.v))(num(2))
    println("c_obj_obj_any " + res + " " + log.mkString(","))
  }
  object O_obj_obj_dropped { log = log :+ "I"; val v = 3; val s = "o" }
  object P_obj_obj_dropped { log = log :+ "J"; val v = 4; val s = "p" }
  def c_obj_obj_dropped(): Unit = {
    log = Nil
    extension (x: Int) def +:(y: String)(z: Int): String = y + x + z
    log = Nil
    (+:(P_obj_obj_dropped.s)(O_obj_obj_dropped.v))(num(2))
    println("c_obj_obj_dropped " + log.mkString(","))
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
    try c_obj_imp_kept() catch { case e: Throwable => println("c_obj_imp_kept !" + e.getClass.getName) }
    try c_obj_imp_any() catch { case e: Throwable => println("c_obj_imp_any !" + e.getClass.getName) }
    try c_obj_imp_dropped() catch { case e: Throwable => println("c_obj_imp_dropped !" + e.getClass.getName) }
    try c_imp_obj_kept() catch { case e: Throwable => println("c_imp_obj_kept !" + e.getClass.getName) }
    try c_imp_obj_any() catch { case e: Throwable => println("c_imp_obj_any !" + e.getClass.getName) }
    try c_imp_obj_dropped() catch { case e: Throwable => println("c_imp_obj_dropped !" + e.getClass.getName) }
    try c_obj_lit_kept() catch { case e: Throwable => println("c_obj_lit_kept !" + e.getClass.getName) }
    try c_obj_lit_any() catch { case e: Throwable => println("c_obj_lit_any !" + e.getClass.getName) }
    try c_obj_lit_dropped() catch { case e: Throwable => println("c_obj_lit_dropped !" + e.getClass.getName) }
    try c_obj_obj_kept() catch { case e: Throwable => println("c_obj_obj_kept !" + e.getClass.getName) }
    try c_obj_obj_any() catch { case e: Throwable => println("c_obj_obj_any !" + e.getClass.getName) }
    try c_obj_obj_dropped() catch { case e: Throwable => println("c_obj_obj_dropped !" + e.getClass.getName) }
  }
}
