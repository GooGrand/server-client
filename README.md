# Доброго времени суток!
Немного запоздал с выполнением узких частей сервера.
Основную проблему составили queue и request delay. Не смог найти подходящего решения на данный момент, но полагаю что необходимо грамотно хранить TcpStream на стороне сервера не создавая handshake. Тем не менее, клиенту на данный момент все равно на ответное пожатие рук и он просто отсылает груду сообщений с ожиданием ответа - поэтому реализовать 2s timeout (несмотря на доступные настройки tcpStream, возможно некорректно использовал) - тоже не удалось.
По возможности буду думать над решением