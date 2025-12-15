"""
FontAtlas Web API
==================

FastAPI application exposing font analysis and pairing functionality.

Run:
    uvicorn src.api.main:app --reload
    
Access:
    - API docs: http://localhost:8000/docs
    - Web UI: http://localhost:8000/
"""

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from fastapi.staticfiles import StaticFiles
from fastapi.responses import HTMLResponse
from pathlib import Path

from api.routers import fonts, pairing, anatomy

# Create FastAPI app
app = FastAPI(
    title="FontAtlas API",
    description="Anatomy-based font pairing system with custom geometry analysis",
    version="0.1.0-phase3",
    docs_url="/docs",
    redoc_url="/redoc"
)

# CORS middleware
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],  # Allow all origins for development
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Include routers
app.include_router(fonts.router, prefix="/api/fonts", tags=["Fonts"])
app.include_router(pairing.router, prefix="/api/pairs", tags=["Pairing"])
app.include_router(anatomy.router, prefix="/api/anatomy", tags=["Anatomy"])

# Static files for web UI
static_dir = Path(__file__).parent / "static"
if static_dir.exists():
    app.mount("/static", StaticFiles(directory=str(static_dir)), name="static")


@app.get("/", response_class=HTMLResponse)
async def root():
    """Serve simple web UI."""
    html_file = static_dir / "index.html"
    
    if html_file.exists():
        return html_file.read_text()
    else:
        return """
        <html>
            <head><title>FontAtlas API</title></head>
            <body>
                <h1>FontAtlas API</h1>
                <p>Welcome to the FontAtlas API!</p>
                <ul>
                    <li><a href="/docs">API Documentation (Swagger)</a></li>
                    <li><a href="/redoc">API Documentation (ReDoc)</a></li>
                </ul>
            </body>
        </html>
        """


@app.get("/api/system/status")
async def system_status():
    """Get system health and statistics."""
    from database.vector_db import FontAtlasVectorDB
    from pathlib import Path
    
    # Check Qdrant
    try:
        db = FontAtlasVectorDB()
        info = db.get_collection_info()
        qdrant_status = {
            "connected": True,
            "points_count": info.get("points_count", 0)
        }
    except Exception as e:
        qdrant_status = {
            "connected": False,
            "error": str(e)
        }
    
    # Count font profiles
    profiles_dir = Path("fonts/samples")
    font_files = list(profiles_dir.glob("*.complete.json"))
    
    return {
        "status": "healthy" if qdrant_status["connected"] else "degraded",
        "qdrant": qdrant_status,
        "fonts": {
            "total": len(font_files),
            "with_anatomy": len(font_files)  # All have anatomy in Phase 3
        },
        "version": "0.1.0-phase3"
    }


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8000)
