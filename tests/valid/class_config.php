<?php

class Config
{
    public function __construct(
        private string $host,
        private int $port,
        private bool $tls = false,
    ) {
    }

    public function dsn(): string
    {
        $scheme = $this->tls ? 'https' : 'http';
        return sprintf('%s://%s:%d', $scheme, $this->host, $this->port);
    }
}

$config = new Config('localhost', 8080, true);
echo $config->dsn();

