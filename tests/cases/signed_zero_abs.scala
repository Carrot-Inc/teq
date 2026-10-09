// Math.abs, scala.math.abs and RichDouble.abs make -0.0 into 0.0; max and min take 0.0 above
// -0.0 and let a NaN win, as Java's do, for Double and Float on every target (Scala.js gives the
// same).
object Main:
  def sgn(d: Double): String = if d != d then "NaN" else if d == 0 then (if 1 / d > 0 then "+0" else "-0") else d.toString
  def sgnf(f: Float): String = if f != f then "NaN" else if f == 0 then (if 1 / f > 0 then "+0" else "-0") else f.toString
  def main(args: Array[String]): Unit =
    val nz = -0.0; val pz = 0.0; val nan = Double.NaN
    val nzf = -0.0f; val pzf = 0.0f; val nanf = 0.0f / 0.0f
    println("Math.abs " + sgn(Math.abs(nz)) + " " + sgn(Math.abs(pz)) + " " + sgn(Math.abs(nan)) + " " + sgn(Math.abs(-2.5)))
    println("math.abs " + sgn(scala.math.abs(nz)) + " " + sgn(scala.math.abs(nan)))
    println("rich abs " + sgn(nz.abs) + " " + sgn(pz.abs) + " " + sgn(nan.abs) + " " + sgn((-3.5).abs))
    println("Math.max " + sgn(Math.max(nz, pz)) + " " + sgn(Math.max(pz, nz)) + " " + sgn(Math.max(nan, 1.0)) + " " + sgn(Math.max(1.0, nan)))
    println("Math.min " + sgn(Math.min(nz, pz)) + " " + sgn(Math.min(pz, nz)) + " " + sgn(Math.min(nan, 1.0)) + " " + sgn(Math.min(1.0, nan)))
    println("math.max " + sgn(scala.math.max(pz, nz)) + " " + sgn(scala.math.min(pz, nz)) + " " + sgn(scala.math.max(nan, 1.0)))
    println("rich max " + sgn(pz.max(nz)) + " " + sgn(nz.max(pz)) + " " + sgn(nan.max(1.0)) + " " + sgn((1.0).max(nan)))
    println("rich min " + sgn(pz.min(nz)) + " " + sgn(nz.min(pz)) + " " + sgn(nan.min(1.0)) + " " + sgn((1.0).min(nan)))
    println("int abs " + Math.abs(Int.MinValue) + " " + Math.abs(-3) + " " + (-3).abs + " " + Math.abs(-3L) + " " + (-7L).abs + " " + Math.abs(Long.MinValue))
    println("int max " + Math.max(3, -4) + " " + Math.min(3L, -4L) + " " + (3).max(9) + " " + (3L).min(-9L))
    println("seq max " + sgn(List(nz, pz).max) + " " + sgn(List(pz, nz).max) + " " + sgn(List(pz, nz).min) + " " + sgn(List(nz, pz).min))
    println("bits " + java.lang.Double.doubleToLongBits(Math.abs(nz)) + " " + java.lang.Double.doubleToLongBits(nz.abs))
    println("signum " + sgn(Math.signum(nz)) + " " + sgn(nz.sign) + " " + sgn(Math.signum(nan)))
    println("float abs " + sgnf(scala.math.abs(nzf)) + " " + sgnf(scala.math.abs(pzf)) + " " + sgnf(scala.math.abs(nanf)) + " " + java.lang.Float.floatToIntBits(scala.math.abs(nzf)))
    println("float max " + sgnf(scala.math.max(pzf, nzf)) + " " + sgnf(scala.math.max(nzf, pzf)) + " " + sgnf(scala.math.min(nzf, pzf)) + " " + sgnf(scala.math.min(pzf, nzf)) + " " + sgnf(scala.math.max(nanf, 1.0f)))
    println("float bits " + java.lang.Float.floatToIntBits(nzf) + " " + java.lang.Float.floatToIntBits(scala.math.max(nzf, pzf)))
