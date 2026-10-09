package lib

trait Throwable:
  def getMessage: String = toString
  def getCause: Option[Throwable] = None

trait Serializable
trait Product2Like[+A, +B]

final class Failure(message: String, cause: Option[Throwable]) extends Throwable:
  override def getMessage: String = message
  override def getCause: Option[Throwable] = cause
  override def toString: String = "Failure: " + message

object Failure:
  def apply(message: String): Failure = new Failure(message, None)
  def apply2(message: String, cause: Throwable): Failure = new Failure(message, Some(cause))
