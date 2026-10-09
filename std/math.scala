package scala.math

export scala.{Fractional, Integral, Numeric, Ordered, Ordering}

val Pi: Double = 3.141592653589793
val E: Double = 2.718281828459045

@js("Math.sqrt($0)")
@jvm("invokestatic java/lang/Math.sqrt(D)D")
def sqrt(x: Double): Double
@js("Math.pow($0, $1)")
@jvm("invokestatic java/lang/Math.pow(DD)D")
def pow(x: Double, y: Double): Double
@js("Math.floor($0)")
@jvm("invokestatic java/lang/Math.floor(D)D")
def floor(x: Double): Double
@js("Math.ceil($0)")
@jvm("invokestatic java/lang/Math.ceil(D)D")
def ceil(x: Double): Double
@js("$d2l(Math.round($0))")
@jvm("invokestatic java/lang/Math.round(D)J")
def round(x: Double): Long
@js("Math.hypot($0, $1)")
@jvm("invokestatic java/lang/Math.hypot(DD)D")
def hypot(x: Double, y: Double): Double
@js("Math.cbrt($0)")
@jvm("invokestatic java/lang/Math.cbrt(D)D")
def cbrt(x: Double): Double
@js("Math.asin($0)")
@jvm("invokestatic java/lang/Math.asin(D)D")
def asin(x: Double): Double
@js("Math.acos($0)")
@jvm("invokestatic java/lang/Math.acos(D)D")
def acos(x: Double): Double
@js("Math.atan($0)")
@jvm("invokestatic java/lang/Math.atan(D)D")
def atan(x: Double): Double
@js("Math.sinh($0)")
@jvm("invokestatic java/lang/Math.sinh(D)D")
def sinh(x: Double): Double
@js("Math.cosh($0)")
@jvm("invokestatic java/lang/Math.cosh(D)D")
def cosh(x: Double): Double
@js("Math.tanh($0)")
@jvm("invokestatic java/lang/Math.tanh(D)D")
def tanh(x: Double): Double
@js("Math.log1p($0)")
@jvm("invokestatic java/lang/Math.log1p(D)D")
def log1p(x: Double): Double
@js("Math.expm1($0)")
@jvm("invokestatic java/lang/Math.expm1(D)D")
def expm1(x: Double): Double
@js("Math.random()")
@jvm("invokestatic java/lang/Math.random()D")
def random(): Double
@js("$signum($0)")
@jvm("rt $0:L rtcall numSignum(Ljava/lang/Object;)Ljava/lang/Object;")
def signum[T <: Int | Long | Float | Double](x: T): T
@js("$floorDiv($0, $1)")
@jvm("rt $0:L $1:L rtcall numFloorDiv(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;")
def floorDiv[T <: Int | Long](a: T, b: T): T
@js("$floorMod($0, $1)")
@jvm("rt $0:L $1:L rtcall numFloorMod(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;")
def floorMod[T <: Int | Long](a: T, b: T): T
def toRadians(degrees: Double): Double = degrees / 180.0 * Pi
def toDegrees(radians: Double): Double = radians * 180.0 / Pi
@js("Math.sin($0)")
@jvm("invokestatic java/lang/Math.sin(D)D")
def sin(x: Double): Double
@js("Math.cos($0)")
@jvm("invokestatic java/lang/Math.cos(D)D")
def cos(x: Double): Double
@js("Math.tan($0)")
@jvm("invokestatic java/lang/Math.tan(D)D")
def tan(x: Double): Double
@js("Math.atan2($0, $1)")
@jvm("invokestatic java/lang/Math.atan2(DD)D")
def atan2(y: Double, x: Double): Double
@js("Math.exp($0)")
@jvm("invokestatic java/lang/Math.exp(D)D")
def exp(x: Double): Double
@js("Math.log($0)")
@jvm("invokestatic java/lang/Math.log(D)D")
def log(x: Double): Double
@js("Math.log10($0)")
@jvm("invokestatic java/lang/Math.log10(D)D")
def log10(x: Double): Double
@js("$max($0, $1)")
@jvm("rt $0:L $1:L rtcall numMax(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;")
def max[T <: Int | Long | Float | Double](a: T, b: T): T
@js("$min($0, $1)")
@jvm("rt $0:L $1:L rtcall numMin(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;")
def min[T <: Int | Long | Float | Double](a: T, b: T): T
@js("$abs($0)")
@jvm("rt $0:L rtcall numAbs(Ljava/lang/Object;)Ljava/lang/Object;")
def abs[T <: Int | Long | Float | Double](x: T): T
@js("$rint($0)")
@jvm("invokestatic java/lang/Math.rint(D)D")
def rint(x: Double): Double
