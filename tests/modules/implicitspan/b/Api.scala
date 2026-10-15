package isp

object Api:
  val from = query[java.time.Instant]("from")
  val to = query[java.time.Instant]("to")
