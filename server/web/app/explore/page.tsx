"use client";

import { useState, useEffect } from "react";
import Link from "next/link";

const API_BASE = "http://localhost:3000/api/v1";

export default function ExplorePage() {
  const [projects, setProjects] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetchProjects();
  }, []);

  const fetchProjects = async () => {
    try {
      setLoading(true);
      const res = await fetch(`${API_BASE}/projects`);
      if (res.ok) {
        setProjects(await res.json());
      }
    } catch (e) {
      console.error("Failed to fetch projects", e);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-gray-50 pb-12">
      <div className="mx-auto max-w-7xl px-4 py-12 sm:px-6 lg:px-8">
        <div className="mb-10 flex items-center justify-between">
          <div>
            <h1 className="text-3xl font-extrabold tracking-tight text-gray-900">
              Explore Projects
            </h1>
            <p className="mt-2 text-base text-gray-600">
              Discover and contribute to binary asset repositories.
            </p>
          </div>
          <Link
            href="/projects/new"
            className="inline-flex items-center rounded-md border border-transparent bg-indigo-600 px-4 py-2 text-sm font-semibold text-white shadow-sm transition-all hover:bg-indigo-700 focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 focus:outline-none"
          >
            New Project
          </Link>
        </div>

        {loading ? (
          <div className="flex justify-center py-20">
            <div className="h-12 w-12 animate-spin rounded-full border-b-2 border-indigo-600"></div>
          </div>
        ) : (
          <div className="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3">
            {projects.map((p) => (
              <div
                key={p.name}
                className="flex flex-col overflow-hidden rounded-lg border border-gray-200 bg-white shadow-sm transition-all hover:border-indigo-300 hover:shadow-md"
              >
                <div className="flex flex-1 flex-col p-6">
                  <div className="flex items-center gap-3">
                    <div className="flex h-10 w-10 items-center justify-center rounded border border-indigo-100 bg-indigo-50 font-bold text-indigo-600">
                      {p.owner_name?.charAt(0).toUpperCase() || "P"}
                    </div>
                    <div className="flex flex-col">
                      <span className="text-xs font-semibold tracking-wider text-indigo-600 uppercase">
                        {p.owner_name}
                      </span>
                      <Link
                        href={`/projects/${p.name}`}
                        className="text-xl font-bold text-gray-900 transition-colors hover:text-indigo-600"
                      >
                        {p.name}
                      </Link>
                    </div>
                  </div>
                  <p className="mt-4 line-clamp-3 text-sm text-gray-600">
                    A project repository for versioning large binary assets and
                    collaborative development.
                  </p>
                  <div className="mt-auto flex items-center justify-between border-t border-gray-50 pt-6">
                    <div className="flex items-center gap-1 text-xs font-medium text-gray-500">
                      <svg
                        className="h-4 w-4"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                      >
                        <path
                          strokeLinecap="round"
                          strokeLinejoin="round"
                          strokeWidth={2}
                          d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"
                        />
                      </svg>
                      Active
                    </div>
                    <Link
                      href={`/projects/${p.name}`}
                      className="group inline-flex items-center text-sm font-semibold text-indigo-600 hover:text-indigo-700"
                    >
                      View Code
                      <svg
                        className="ml-1 h-4 w-4 transform transition-transform group-hover:translate-x-1"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                      >
                        <path
                          strokeLinecap="round"
                          strokeLinejoin="round"
                          strokeWidth={2}
                          d="M9 5l7 7-7 7"
                        />
                      </svg>
                    </Link>
                  </div>
                </div>
              </div>
            ))}

            {projects.length === 0 && (
              <div className="col-span-full rounded-xl border-2 border-dashed border-gray-200 bg-white py-24 text-center">
                <div className="mx-auto h-12 w-12 text-gray-300">
                  <svg fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={1.5}
                      d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"
                    />
                  </svg>
                </div>
                <h3 className="mt-4 text-base font-semibold text-gray-900">
                  No projects found
                </h3>
                <p className="mt-1 text-sm text-gray-500">
                  Be the first to create a project on Rig.
                </p>
                <Link
                  href="/projects/new"
                  className="mt-6 inline-flex items-center px-4 py-2 text-sm font-semibold text-indigo-600 hover:text-indigo-700"
                >
                  Create Project &rarr;
                </Link>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
