#!/bin/bash

# Local development setup script for BlocChat Backend

set -e

echo "🚀 BlocChat Backend - Local Development Setup"
echo ""

# Check if .env exists
if [ ! -f .env ]; then
    echo "❌ .env file not found. Creating from template..."
    cp .env.example .env
    echo "⚠️  Please edit .env with your configuration before running the backend"
    exit 1
fi

# Check if PostgreSQL is running
if ! pg_isready -q 2>/dev/null; then
    echo "❌ PostgreSQL is not running. Please start PostgreSQL:"
    echo "   macOS: brew services start postgresql@14"
    echo "   Linux: sudo systemctl start postgresql"
    exit 1
fi

# Check if database exists
if ! psql -lqt | cut -d \| -f 1 | grep -qw blocchat 2>/dev/null; then
    echo "📦 Creating database 'blocchat'..."
    createdb blocchat || {
        echo "⚠️  Could not create database. You may need to:"
        echo "   1. Ensure PostgreSQL is running"
        echo "   2. Check your PostgreSQL user permissions"
        exit 1
    }
fi

# Run migrations
echo "🔄 Running database migrations..."
psql -d blocchat -f migrations/001_create_transactions.sql || {
    echo "⚠️  Migrations failed. The tables may already exist (this is OK)"
}

echo ""
echo "✅ Setup complete!"
echo ""
echo "Starting backend in development mode..."
echo "Press Ctrl+C to stop"
echo ""

# Run backend
cargo run
