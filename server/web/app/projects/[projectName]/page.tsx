"use client";

import { useState, useEffect } from "react";
import { useParams } from "next/navigation";
import Link from "next/link";

const API_BASE = "http://localhost:3000/api/v1";

interface Project {
  name: string;
  owner_id: string;
  owner_name?: string;
  clone_url_http?: string;
}

interface User {
  id: string;
  name: string;
  email: string;
}

interface Permission {
  user_id: string;
  project: string;
  access: "read" | "write" | "admin";
}

interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  locked_by: string | null;
}

export default function ProjectPage() {
  const params = useParams<{ projectName: string }>();
  const projectName = params.projectName;

  const [project, setProject] = useState<Project | null>(null);
  const [users, setUsers] = useState<User[]>([]);
  const [permissions, setPermissions] = useState<Permission[]>([]);
  const [currentUser, setCurrentUser] = useState<User | null>(null);
  const [isAdmin, setIsAdmin] = useState(false);
  const [token, setToken] = useState("");

  const [files, setFiles] = useState<FileEntry[]>([]);
  const [currentPath, setCurrentPath] = useState("");
  const [isLoadingFiles, setIsLoadingFiles] = useState(false);

  useEffect(() => {
    const t = localStorage.getItem("token");
    if (t) {
      setToken(t);
      // Fetch current user details to check ownership/admin status
      fetch(`${API_BASE}/users/me`, {
        headers: { Authorization: `Bearer ${t}` },
      })
        .then((res) => {
          if (!res.ok) throw new Error("Authentication failed");
          return res.json();
        })
        .then((data) => {
          setCurrentUser(data);
          fetchProjectAndPermissions(t, data.id);
        })
        .catch((error) => {
          console.error("Failed to fetch current user:", error);
          fetchProjectAndPermissions(t, null); // Try fetching without user ID if auth fails
        });
    } else {
      fetchProjectAndPermissions(null, null); // Fetch project details even if not logged in
    }
  }, [projectName]); // Re-fetch when projectName changes

  const fetchProjectAndPermissions = async (
    authToken: string | null,
    currentUserId: string | null,
  ) => {
    try {
      // Fetch project details
      const projectRes = await fetch(`${API_BASE}/projects/${projectName}`, {
        headers: authToken ? { Authorization: `Bearer ${authToken}` } : {},
      });
      if (!projectRes.ok) {
        // Handle 404 or other errors if project not found
        if (projectRes.status === 404) {
          console.error("Project not found");
          // Optionally redirect or show a "not found" message
        } else {
          throw new Error(`Failed to fetch project: ${projectRes.statusText}`);
        }
        return;
      }
      const projectData: Project = await projectRes.json();
      const ownerName = projectData.owner_name || "User";
      projectData.clone_url_http = `http://localhost:3000/${ownerName}/${projectData.name}`;
      setProject(projectData);

      // Fetch all users and permissions if logged in
      if (authToken && currentUserId) {
        const [usersRes, permsRes] = await Promise.all([
          fetch(`${API_BASE}/users`, {
            headers: { Authorization: `Bearer ${authToken}` },
          }),
          fetch(`${API_BASE}/permissions`, {
            headers: { Authorization: `Bearer ${authToken}` },
          }),
        ]);
        const fetchedUsers = await usersRes.json();
        const fetchedPermissions = await permsRes.json();

        setUsers(fetchedUsers);
        setPermissions(fetchedPermissions);

        // Check if current user is admin for this project
        const userPermissions = fetchedPermissions.filter(
          (p: Permission) =>
            p.user_id === currentUserId && p.project === projectName,
        );
        const isAdminUser = userPermissions.some(
          (p: Permission) => p.access === "admin",
        );
        const isOwner = projectData.owner_id === currentUserId;
        setIsAdmin(isAdminUser || isOwner);

        // Initial fetch of root files
        fetchFiles(authToken, "");
      }
    } catch (e) {
      console.error("Failed to fetch data", e);
    }
  };

  const fetchFiles = async (authToken: string, path: string) => {
    setIsLoadingFiles(true);
    try {
      const url = new URL(`${API_BASE}/projects/${projectName}/files`);
      if (path) {
        url.searchParams.append("path", path);
      }
      const res = await fetch(url.toString(), {
        headers: { Authorization: `Bearer ${authToken}` },
      });
      if (res.ok) {
        setFiles(await res.json());
        setCurrentPath(path);
      }
    } catch (e) {
      console.error("Failed to load files", e);
    } finally {
      setIsLoadingFiles(false);
    }
  };

  const navigateToDirectory = (path: string) => {
    const t = localStorage.getItem("token");
    if (t) fetchFiles(t, path);
  };

  const handleBreadcrumbClick = (index: number) => {
    const parts = currentPath.split("/").filter(Boolean);
    const newPath = parts.slice(0, index + 1).join("/");
    navigateToDirectory(newPath);
  };

  if (!project) {
    // Show a loading or not found message
    return (
      <div className="flex min-h-screen items-center justify-center bg-gray-50">
        <p className="text-lg text-gray-700">
          {projectName ? `Loading project "${projectName}"...` : "Loading..."}
        </p>
      </div>
    );
  }

  return (
    <div className="min-h-[calc(100vh-56px)] bg-gray-50">
      {/* Project Header (Gitea style) */}
      <div className="border-b border-gray-200 bg-white">
        <div className="mx-auto max-w-6xl px-4 py-5 sm:px-6 lg:px-8">
          <div className="mb-4 flex items-center space-x-2 text-xl">
            <div className="flex h-6 w-6 items-center justify-center rounded bg-indigo-100 text-xs font-bold text-indigo-700">
              {project.owner_id.charAt(0).toUpperCase()}
            </div>
            <span className="cursor-pointer font-medium text-gray-500 hover:text-indigo-600 hover:underline">
              {project.owner_name || "User"}
            </span>
            <span className="text-gray-400">/</span>
            <span className="font-bold text-gray-900">{project.name}</span>
          </div>

          <nav className="flex space-x-6 text-sm font-medium">
            <Link
              href={`/projects/${projectName}`}
              className="border-b-2 border-indigo-500 px-2 py-3 text-gray-900"
            >
              <span className="flex items-center gap-2">
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
                    d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4"
                  />
                </svg>
                Code
              </span>
            </Link>
            {isAdmin && (
              <Link
                href={`/projects/${projectName}/settings`}
                className="border-b-2 border-transparent px-2 py-3 text-gray-500 hover:border-gray-300 hover:text-gray-700"
              >
                <span className="flex items-center gap-2">
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
                      d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
                    />
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                    />
                  </svg>
                  Settings
                </span>
              </Link>
            )}
          </nav>
        </div>
      </div>

      <div className="mx-auto max-w-6xl px-4 py-8 sm:px-6 lg:px-8">
        {/* Clone Section */}
        <div className="mb-6 rounded-md border border-gray-200 bg-white">
          <div className="flex items-center justify-between px-4 py-3 sm:px-6">
            <h3 className="text-base leading-6 font-semibold text-gray-900">
              Clone Repository
            </h3>
          </div>
          {project.clone_url_http && (
            <div className="space-y-3 border-t border-gray-200 px-4 py-4 sm:px-6">
              <div className="flex items-center justify-between rounded border border-gray-300 bg-gray-50 p-3">
                <span className="mr-4 w-12 text-xs font-bold tracking-wider text-gray-500 uppercase">
                  HTTP
                </span>
                <p className="flex-1 overflow-auto font-mono text-sm text-gray-800">
                  rig clone {project.clone_url_http}
                </p>
                <button
                  onClick={() =>
                    navigator.clipboard.writeText(
                      `rig clone ${project.clone_url_http}`,
                    )
                  }
                  className="ml-4 rounded-md border border-indigo-200 bg-indigo-50 px-3 py-1 text-sm font-semibold text-indigo-700 transition-colors hover:bg-indigo-100 focus:outline-none"
                >
                  Copy
                </button>
              </div>
            </div>
          )}
        </div>

        {/* File Explorer Section */}
        <div className="rounded-md border border-gray-200 bg-white">
          <div className="flex items-center justify-between rounded-t-md border-b border-gray-200 bg-gray-50 px-4 py-3">
            <div className="flex items-center text-sm font-semibold text-gray-700">
              <button
                onClick={() => navigateToDirectory("")}
                className="transition-colors hover:text-indigo-600"
              >
                {project.name}
              </button>
              {currentPath
                .split("/")
                .filter(Boolean)
                .map((part, index) => (
                  <span key={index} className="flex items-center">
                    <span className="mx-2 text-gray-400">/</span>
                    <button
                      onClick={() => handleBreadcrumbClick(index)}
                      className="transition-colors hover:text-indigo-600"
                    >
                      {part}
                    </button>
                  </span>
                ))}
            </div>
            {isLoadingFiles && (
              <div className="h-4 w-4 animate-spin rounded-full border-b-2 border-indigo-600"></div>
            )}
          </div>

          <div className="bg-white">
            <ul role="list" className="divide-y divide-gray-100">
              {currentPath !== "" && (
                <li
                  className="group flex cursor-pointer items-center bg-white px-4 py-3 transition-colors hover:bg-gray-50"
                  onClick={() => {
                    const parts = currentPath.split("/").filter(Boolean);
                    parts.pop();
                    navigateToDirectory(parts.join("/"));
                  }}
                >
                  <svg
                    className="mr-3 h-5 w-5 text-gray-400"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M3 10h10a8 8 0 018 8v2M3 10l6 6m-6-6l6-6"
                    />
                  </svg>
                  <span className="text-sm font-medium text-gray-700 group-hover:text-indigo-600">
                    ..
                  </span>
                </li>
              )}

              {files.map((file) => (
                <li
                  key={file.path}
                  className={`flex items-center justify-between bg-white px-4 py-3 transition-colors hover:bg-gray-50 ${file.is_dir ? "group cursor-pointer" : ""}`}
                  onClick={() => file.is_dir && navigateToDirectory(file.path)}
                >
                  <div className="flex min-w-0 items-center justify-start gap-x-3">
                    {file.is_dir ? (
                      <svg
                        className="h-5 w-5 flex-none text-blue-400"
                        viewBox="0 0 20 20"
                        fill="currentColor"
                      >
                        <path d="M2 6a2 2 0 012-2h4l2 2h6a2 2 0 012 2v8a2 2 0 01-2 2H4a2 2 0 01-2-2V6z" />
                      </svg>
                    ) : (
                      <svg
                        className="h-5 w-5 flex-none text-gray-400"
                        viewBox="0 0 20 20"
                        fill="currentColor"
                      >
                        <path
                          fillRule="evenodd"
                          d="M4 4a2 2 0 012-2h4.586A2 2 0 0112 2.586L15.414 6A2 2 0 0116 7.414V16a2 2 0 01-2 2H6a2 2 0 01-2-2V4zm2 6a1 1 0 011-1h6a1 1 0 110 2H7a1 1 0 01-1-1zm1 3a1 1 0 100 2h6a1 1 0 100-2H7z"
                          clipRule="evenodd"
                        />
                      </svg>
                    )}
                    <span
                      className={`truncate text-sm ${file.is_dir ? "font-medium text-gray-900 group-hover:text-indigo-600" : "text-gray-700"}`}
                    >
                      {file.name}
                    </span>
                  </div>

                  <div className="flex shrink-0 items-center justify-end">
                    {file.locked_by && (
                      <span className="inline-flex items-center gap-x-1.5 rounded-md bg-red-50 px-2 py-1 text-xs font-medium text-red-700 ring-1 ring-red-600/20 ring-inset">
                        <svg
                          className="h-3.5 w-3.5"
                          viewBox="0 0 20 20"
                          fill="currentColor"
                        >
                          <path
                            fillRule="evenodd"
                            d="M5 9V7a5 5 0 0110 0v2a2 2 0 012 2v5a2 2 0 01-2 2H5a2 2 0 01-2-2v-5a2 2 0 012-2zm8-2v2H7V7a3 3 0 016 0z"
                            clipRule="evenodd"
                          />
                        </svg>
                        Locked by {file.locked_by}
                      </span>
                    )}
                  </div>
                </li>
              ))}

              {files.length === 0 && !isLoadingFiles && (
                <li className="px-4 py-8 text-center text-sm text-gray-500">
                  This directory is empty.
                </li>
              )}
            </ul>
          </div>
        </div>
      </div>
    </div>
  );
}
