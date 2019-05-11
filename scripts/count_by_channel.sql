SELECT channel, COUNT(channel) FROM messages GROUP BY channel ORDER BY COUNT(channel) DESC;
