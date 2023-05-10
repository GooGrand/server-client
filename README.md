# Доброго времени суток!
Немного запоздал с выполнением узких частей сервера.
Основную проблему составили queue и request delay. Не смог найти подходящего решения на данный момент, но полагаю что необходимо грамотно хранить TcpStream на стороне сервера не создавая handshake. Тем не менее, клиенту на данный момент все равно на ответное пожатие рук и он просто отсылает груду сообщений с ожиданием ответа, поэтому реализовать 2s timeout (несмотря на доступные настройки tcpStream, возможно некорректно использовал) - пока не удалось.
По возможности буду думать над решением

# Apology
Хотелось бы сказать, что до этого тестового у меня не было нормального опыта работы с service-client приложениями, все ограничивалось фреймворками (actix, вшитые манипуляции блокчейн фреймворков). Поэтому путь исследования пошел через растбук в h2. Растбук дал структуру ООП которую было сложно превратить в то, что было нужно.
В связи с этим было принято решение оставить какого то рода скриптовый подход, который сохранился и до сих пор (хотя вижу варианты создания хороших сущностей с нормальными временами жизни). 
Тем не менее результат - это работа всего двух дней (в которые входят метания по архитектуре и по библиотекам). 