"use client";

import { useState, useEffect } from "react";

const API_BASE = "http://localhost:3000/api/v1";

export default function Home() {
  const [projects, setProjects] = useState<any[]>([]);
  const [isLoggedIn, setIsLoggedIn] = useState(false);
  const [token, setToken] = useState("");

  const [newProjectName, setNewProjectName] = useState("");

  useEffect(() => {
    const t = localStorage.getItem("token");
    if (t) {
      setToken(t);
      setIsLoggedIn(true);
    }
    fetchProjects();
  }, [isLoggedIn]); // Re-fetch projects if login status changes

  const fetchProjects = async () => {
    try {
      const res = await fetch(`${API_BASE}/projects`);
      if (res.ok) {
        setProjects(await res.json());
      }
    } catch (e) {
      console.error("Failed to fetch projects", e);
    }
  };

  const addProject = async (e: React.FormEvent) => {
    e.preventDefault();
    const res = await fetch(`${API_BASE}/create_project`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${token}`,
      },
      body: JSON.stringify({ name: newProjectName }),
    });
    if (res.ok) {
      setNewProjectName("");
      fetchProjects();
    } else {
      const data = await res.json();
      alert(`Failed to create project: ${data.message || res.statusText}`);
    }
  };

  return (
    <div className="min-h-screen bg-gray-50">
      <div className="mx-auto max-w-7xl py-6 sm:px-6 lg:px-8">
        <div className="px-4 py-6 sm:px-0">
          <h1 className="mb-8 text-3xl font-bold text-gray-900">Projects</h1>

          <div className="mb-8 grid grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-3">
            {projects.map((p) => (
              <div
                key={p.name}
                className="overflow-hidden rounded-lg bg-white shadow transition-shadow hover:shadow-md"
              >
                <div className="p-6">
                  <h3 className="text-lg font-medium text-gray-900">
                    {p.name}
                  </h3>
                  <p className="mt-2 text-sm text-gray-500">
                    A project repository
                  </p>
                  <div className="mt-4">
                    <a
                      href={`/projects/${p.name}`}
                      className="text-sm font-medium text-indigo-600 hover:text-indigo-500"
                    >
                      View project →
                    </a>
                  </div>
                </div>
              </div>
            ))}
            {projects.length === 0 && (
              <p className="col-span-full py-12 text-center text-gray-500">
                No projects found.
              </p>
            )}
          </div>

          {isLoggedIn && (
            <div className="rounded-lg bg-white p-6 shadow">
              <h2 className="mb-4 text-xl font-semibold text-gray-900">
                Create New Project
              </h2>
              <form onSubmit={addProject} className="flex gap-4">
                <input
                  type="text"
                  placeholder="Project Name"
                  value={newProjectName}
                  onChange={(e) => setNewProjectName(e.target.value)}
                  required
                  className="flex-1 rounded-md border border-gray-300 px-3 py-2 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 focus:outline-none"
                />
                <button
                  type="submit"
                  className="rounded-md bg-indigo-600 px-4 py-2 text-white transition-colors hover:bg-indigo-700 focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 focus:outline-none"
                >
                  Create Project
                </button>
              </form>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
